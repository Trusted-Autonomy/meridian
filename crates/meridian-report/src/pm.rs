use anyhow::Result;
use meridian_core::metric::Metric;
use serde::{Deserialize, Serialize};

use crate::auth::{call_claude, extract_json, AuthMethod};

// ── Interview ──────────────────────────────────────────────────────────────────

/// Structured answers from the PM interview session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PmInterview {
    /// Strategic priorities the user cares about (e.g. "reduce time-to-ship").
    pub priorities: Vec<String>,
    /// Loose goals stated in user's own words.
    pub goals: Vec<String>,
    /// Constraints to respect (budget, team size, tech limitations).
    pub constraints: Vec<String>,
    /// Approximate team size, used to calibrate KPI ambition.
    pub team_size: Option<u32>,
    /// Industry vertical (e.g. "SaaS", "game studio", "fintech").
    pub industry: Option<String>,
}

// ── Draft KPI ──────────────────────────────────────────────────────────────────

/// A KPI with metrics proposed by the PM generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftKpi {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    /// Why this KPI was suggested given the interview answers.
    pub rationale: String,
}

fn default_weight() -> f32 {
    1.0
}

// ── Red team panelists ─────────────────────────────────────────────────────────

/// A panelist in the red team review.
pub struct Panelist {
    pub role: &'static str,
    pub perspective: &'static str,
}

/// The seven red-team panelists who challenge the draft KPIs.
pub const PANELISTS: &[Panelist] = &[
    Panelist {
        role: "CEO",
        perspective: "company-wide strategic alignment and investor narrative",
    },
    Panelist {
        role: "CFO",
        perspective: "financial viability, cost controls, and ROI measurability",
    },
    Panelist {
        role: "Head of Product",
        perspective: "customer value, product-market fit, and user outcomes",
    },
    Panelist {
        role: "Tech Director",
        perspective: "technical feasibility, engineering capacity, and maintainability",
    },
    Panelist {
        role: "Lead Security Engineer",
        perspective: "security risk, compliance obligations, and data governance",
    },
    Panelist {
        role: "Head of Marketing",
        perspective: "brand perception, market positioning, and external communication",
    },
    Panelist {
        role: "Head of Sales",
        perspective: "revenue impact, customer acquisition, and deal velocity",
    },
];

// ── Red-team verdict types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcernSeverity {
    High,
    Medium,
    Low,
}

impl ConcernSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelistConcern {
    pub concern: String,
    pub severity: ConcernSeverity,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelistDecision {
    Approve,
    Challenge,
    Reject,
}

impl PanelistDecision {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::Challenge => "CHALLENGE",
            Self::Reject => "REJECT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelistVerdict {
    pub panelist_role: String,
    pub decision: PanelistDecision,
    pub concerns: Vec<PanelistConcern>,
}

#[derive(Debug, Clone)]
pub struct ConsensusVerdict {
    pub approved: usize,
    pub challenged: usize,
    pub rejected: usize,
    /// High-severity concerns across all panelists.
    pub high_severity: Vec<(String, PanelistConcern)>,
}

// ── Raw wire types for JSON parsing ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawDraftKpi {
    id: String,
    label: String,
    description: String,
    #[serde(default = "default_weight")]
    weight: f32,
    #[serde(default)]
    metrics: Vec<RawMetric>,
    #[serde(default)]
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct RawMetric {
    id: String,
    label: String,
    unit: String,
    target: String,
    source: String,
    frequency: String,
}

#[derive(Debug, Deserialize)]
struct RawVerdict {
    decision: String,
    #[serde(default)]
    concerns: Vec<RawConcern>,
}

#[derive(Debug, Deserialize)]
struct RawConcern {
    concern: String,
    #[serde(default = "default_severity_str")]
    severity: String,
    #[serde(default)]
    suggested_fix: Option<String>,
}

fn default_severity_str() -> String {
    "Medium".to_string()
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Generate an initial set of KPIs with metrics from the PM interview answers.
pub fn generate_draft_kpis(
    interview: &PmInterview,
    auth: &AuthMethod,
    model: &str,
) -> Result<Vec<DraftKpi>> {
    let prompt = build_draft_prompt(interview);
    let raw = call_claude(&prompt, auth, model, 2048)?;
    parse_draft_kpis(&raw)
}

/// Run the 7-panelist red team review of the draft KPIs.
/// Each panelist is prompted independently; results are collected sequentially.
pub fn run_red_team(
    kpis: &[DraftKpi],
    interview: &PmInterview,
    auth: &AuthMethod,
    model: &str,
) -> Result<Vec<PanelistVerdict>> {
    let kpi_summary = format_kpi_summary(kpis);
    let interview_context = format_interview_context(interview);

    let mut verdicts = Vec::with_capacity(PANELISTS.len());
    for p in PANELISTS {
        let prompt = build_panelist_prompt(p, &kpi_summary, &interview_context);
        let raw = call_claude(&prompt, auth, model, 1024)?;
        let verdict = parse_panelist_verdict(p.role, &raw)?;
        verdicts.push(verdict);
    }
    Ok(verdicts)
}

/// Aggregate red team verdicts into a consensus summary.
pub fn consensus(verdicts: &[PanelistVerdict]) -> ConsensusVerdict {
    let mut approved = 0;
    let mut challenged = 0;
    let mut rejected = 0;
    let mut high_severity: Vec<(String, PanelistConcern)> = Vec::new();

    for v in verdicts {
        match v.decision {
            PanelistDecision::Approve => approved += 1,
            PanelistDecision::Challenge => challenged += 1,
            PanelistDecision::Reject => rejected += 1,
        }
        for c in &v.concerns {
            if c.severity == ConcernSeverity::High {
                high_severity.push((v.panelist_role.clone(), c.clone()));
            }
        }
    }

    ConsensusVerdict {
        approved,
        challenged,
        rejected,
        high_severity,
    }
}

/// Regenerate KPIs after the user has responded to red-team concerns.
///
/// `resolutions` is a list of `(concern_text, user_response)` pairs from the
/// interactive review step.
pub fn finalize_with_resolutions(
    kpis: &[DraftKpi],
    resolutions: &[(String, String)],
    auth: &AuthMethod,
    model: &str,
) -> Result<Vec<DraftKpi>> {
    let prompt = build_finalize_prompt(kpis, resolutions);
    let raw = call_claude(&prompt, auth, model, 2048)?;
    parse_draft_kpis(&raw)
}

// ── Prompt builders ────────────────────────────────────────────────────────────

fn build_draft_prompt(i: &PmInterview) -> String {
    let priorities = if i.priorities.is_empty() {
        "(none specified)".to_string()
    } else {
        i.priorities
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let goals = if i.goals.is_empty() {
        "(none specified)".to_string()
    } else {
        i.goals
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let constraints = if i.constraints.is_empty() {
        "(none specified)".to_string()
    } else {
        i.constraints
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let team = i
        .team_size
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let industry = i.industry.as_deref().unwrap_or("not specified");

    format!(
        r#"You are a strategic product advisor helping a team define measurable KPIs.

Team context:
- Industry: {industry}
- Team size: {team}
- Priorities:
{priorities}
- Goals:
{goals}
- Constraints:
{constraints}

Generate 4-6 concrete KPIs that this team should track. For each KPI provide 2-3 measurable metrics.

Respond with ONLY a JSON array — no explanation before or after. Each element must have:
{{
  "id": "snake_case_identifier",
  "label": "Short Human Label",
  "description": "One sentence explaining what this KPI measures and why it matters",
  "weight": 1.0,
  "rationale": "Why this KPI is important given the team's priorities",
  "metrics": [
    {{
      "id": "snake_case_metric_id",
      "label": "Metric Label",
      "unit": "hours|count|percent|dollars|days|ratio",
      "target": "threshold expression like '<3' or '>90%' or '<$5000'",
      "source": "git|github|jira|manual|ta_velocity|slack|survey",
      "frequency": "daily|weekly|monthly|quarterly"
    }}
  ]
}}"#,
        industry = industry,
        team = team,
        priorities = priorities,
        goals = goals,
        constraints = constraints,
    )
}

fn build_panelist_prompt(
    panelist: &Panelist,
    kpi_summary: &str,
    interview_context: &str,
) -> String {
    format!(
        r#"You are the {role} reviewing proposed KPIs from the perspective of {perspective}.

Team background:
{context}

Proposed KPIs:
{kpis}

Review these KPIs critically from your perspective. Identify gaps, risks, or misalignments.

Respond with ONLY a JSON object — no explanation before or after:
{{
  "decision": "Approve" | "Challenge" | "Reject",
  "concerns": [
    {{
      "concern": "Specific concern or gap",
      "severity": "High" | "Medium" | "Low",
      "suggested_fix": "Concrete suggestion to address this concern (optional)"
    }}
  ]
}}

Use "Approve" if the KPIs look good from your perspective (you may still include Low concerns).
Use "Challenge" if there are gaps that should be addressed before finalizing.
Use "Reject" if the KPIs miss something critical from your perspective."#,
        role = panelist.role,
        perspective = panelist.perspective,
        context = interview_context,
        kpis = kpi_summary,
    )
}

fn build_finalize_prompt(kpis: &[DraftKpi], resolutions: &[(String, String)]) -> String {
    let kpi_json = serde_json::to_string_pretty(kpis).unwrap_or_default();
    let resolution_list = resolutions
        .iter()
        .map(|(concern, response)| format!("  Concern: {concern}\n  Response: {response}"))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"You are finalizing a set of KPIs after a red-team review.

Original KPIs:
{kpis}

Concerns raised and team responses:
{resolutions}

Revise the KPIs to incorporate the team's responses to the concerns. Keep the same overall
structure but update descriptions, metrics, weights, or rationale as needed.

Respond with ONLY a JSON array in the same format as the input — no explanation before or after."#,
        kpis = kpi_json,
        resolutions = resolution_list,
    )
}

// ── Formatting helpers ─────────────────────────────────────────────────────────

fn format_kpi_summary(kpis: &[DraftKpi]) -> String {
    kpis.iter()
        .map(|k| {
            let metrics = k
                .metrics
                .iter()
                .map(|m| {
                    format!(
                        "    - {} ({} {}, target {})",
                        m.label, m.unit, m.frequency, m.target
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "  KPI: {} — {}\n  Rationale: {}\n  Metrics:\n{}",
                k.label, k.description, k.rationale, metrics
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_interview_context(i: &PmInterview) -> String {
    let mut parts = Vec::new();
    if let Some(ind) = &i.industry {
        parts.push(format!("Industry: {ind}"));
    }
    if let Some(n) = i.team_size {
        parts.push(format!("Team size: {n}"));
    }
    if !i.priorities.is_empty() {
        parts.push(format!("Priorities: {}", i.priorities.join("; ")));
    }
    if !i.goals.is_empty() {
        parts.push(format!("Goals: {}", i.goals.join("; ")));
    }
    if !i.constraints.is_empty() {
        parts.push(format!("Constraints: {}", i.constraints.join("; ")));
    }
    if parts.is_empty() {
        "(no context provided)".to_string()
    } else {
        parts.join("\n")
    }
}

// ── Parsers ────────────────────────────────────────────────────────────────────

fn parse_draft_kpis(raw: &str) -> Result<Vec<DraftKpi>> {
    let json = extract_json(raw);
    let items: Vec<RawDraftKpi> = serde_json::from_str(json).map_err(|e| {
        anyhow::anyhow!("Failed to parse KPI JSON from Claude response: {e}\nRaw:\n{raw}")
    })?;

    Ok(items
        .into_iter()
        .map(|r| DraftKpi {
            id: r.id,
            label: r.label,
            description: r.description,
            weight: r.weight,
            rationale: r.rationale,
            metrics: r
                .metrics
                .into_iter()
                .map(|m| Metric {
                    id: m.id,
                    label: m.label,
                    unit: m.unit,
                    target: m.target,
                    source: m.source,
                    frequency: m.frequency,
                })
                .collect(),
        })
        .collect())
}

fn parse_panelist_verdict(role: &str, raw: &str) -> Result<PanelistVerdict> {
    let json = extract_json(raw);
    let rv: RawVerdict = serde_json::from_str(json).map_err(|e| {
        anyhow::anyhow!("Failed to parse verdict JSON from {role}: {e}\nRaw:\n{raw}")
    })?;

    let decision = match rv.decision.to_lowercase().as_str() {
        "approve" => PanelistDecision::Approve,
        "reject" => PanelistDecision::Reject,
        _ => PanelistDecision::Challenge,
    };

    let concerns = rv
        .concerns
        .into_iter()
        .map(|c| PanelistConcern {
            concern: c.concern,
            severity: match c.severity.to_lowercase().as_str() {
                "high" => ConcernSeverity::High,
                "low" => ConcernSeverity::Low,
                _ => ConcernSeverity::Medium,
            },
            suggested_fix: c.suggested_fix,
        })
        .collect();

    Ok(PanelistVerdict {
        panelist_role: role.to_string(),
        decision,
        concerns,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_draft_kpis_from_valid_json() {
        let json = r#"[
          {
            "id": "eng_velocity",
            "label": "Engineering Velocity",
            "description": "Ship quality features faster",
            "weight": 1.0,
            "rationale": "Team wants to reduce cycle time",
            "metrics": [
              {
                "id": "cycle_time",
                "label": "Cycle Time",
                "unit": "days",
                "target": "<3",
                "source": "git",
                "frequency": "weekly"
              }
            ]
          }
        ]"#;
        let kpis = parse_draft_kpis(json).unwrap();
        assert_eq!(kpis.len(), 1);
        assert_eq!(kpis[0].id, "eng_velocity");
        assert_eq!(kpis[0].metrics.len(), 1);
        assert_eq!(kpis[0].metrics[0].unit, "days");
    }

    #[test]
    fn parse_draft_kpis_from_fenced_json() {
        let raw = "Sure! Here are your KPIs:\n```json\n[{\"id\":\"x\",\"label\":\"X\",\"description\":\"d\",\"weight\":1.0,\"rationale\":\"r\",\"metrics\":[]}]\n```";
        let kpis = parse_draft_kpis(raw).unwrap();
        assert_eq!(kpis.len(), 1);
    }

    #[test]
    fn parse_panelist_verdict_approve() {
        let json = r#"{"decision":"Approve","concerns":[]}"#;
        let v = parse_panelist_verdict("CEO", json).unwrap();
        assert_eq!(v.decision, PanelistDecision::Approve);
        assert!(v.concerns.is_empty());
    }

    #[test]
    fn parse_panelist_verdict_challenge_with_concerns() {
        let json = r#"{
          "decision": "Challenge",
          "concerns": [
            {"concern": "No customer KPI", "severity": "High", "suggested_fix": "Add NPS"}
          ]
        }"#;
        let v = parse_panelist_verdict("Head of Product", json).unwrap();
        assert_eq!(v.decision, PanelistDecision::Challenge);
        assert_eq!(v.concerns[0].severity, ConcernSeverity::High);
    }

    #[test]
    fn consensus_counts() {
        let verdicts = vec![
            PanelistVerdict {
                panelist_role: "CEO".into(),
                decision: PanelistDecision::Approve,
                concerns: vec![],
            },
            PanelistVerdict {
                panelist_role: "CFO".into(),
                decision: PanelistDecision::Challenge,
                concerns: vec![PanelistConcern {
                    concern: "No cost KPI".into(),
                    severity: ConcernSeverity::High,
                    suggested_fix: None,
                }],
            },
            PanelistVerdict {
                panelist_role: "Head of Product".into(),
                decision: PanelistDecision::Reject,
                concerns: vec![],
            },
        ];
        let c = consensus(&verdicts);
        assert_eq!(c.approved, 1);
        assert_eq!(c.challenged, 1);
        assert_eq!(c.rejected, 1);
        assert_eq!(c.high_severity.len(), 1);
    }
}
