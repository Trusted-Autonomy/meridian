use anyhow::Result;
use clap::Args;
use meridian_config::MeridianConfig;
use meridian_core::record::UsageRecord;
use meridian_core::scorer::KeywordScorer;
use meridian_ingest::{claude_code, generic, ta::TaSource};
use meridian_report::panel::PanelScorer;
use meridian_report::suggest as suggest_lib;
use meridian_report::summary;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DISSENT_THRESHOLD: f32 = 0.20;

#[derive(Args)]
pub struct ServeArgs {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReportParams {
    /// Time window: e.g. "7d", "30d", "2w", "2026-06-01" (default: 7d)
    #[serde(default = "default_since")]
    since: String,
    /// Source type: ta, jsonl, claude-code
    source: Option<String>,
    /// Path to data file or TA project root
    path: Option<String>,
}
fn default_since() -> String {
    "7d".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    source: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KpisParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuggestParams {
    source: Option<String>,
    path: Option<String>,
    /// KPI alignment threshold for flagging low pairs (default: from config)
    threshold: Option<f32>,
}

pub struct MeridianMcpServer {
    config_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MeridianMcpServer {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Run expert panel KPI alignment report. Returns session-by-session consensus scores, per-KPI alignment, panel period averages, and high-dissent flags."
    )]
    fn meridian_report(
        &self,
        Parameters(params): Parameters<ReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = MeridianConfig::load_or_default(&self.config_path);
        let since = super::report::parse_since(&params.since).map_err(mcp_err)?;
        let panel = config.panel_effective();
        let taxonomy = config.taxonomy();

        let records = load_records(
            params.source.as_deref(),
            params.path.as_deref().map(Path::new),
            &config,
        )
        .map_err(mcp_err)?;

        let records: Vec<UsageRecord> = records
            .into_iter()
            .filter(|r| r.timestamp >= since)
            .collect();

        if records.is_empty() {
            return Ok(text_result(format!(
                "No records found in the '{}' window.",
                params.since
            )));
        }

        let scorer = PanelScorer::new(&panel);
        let results = scorer.score_batch(&records);

        let kpi_scorer = if taxonomy.kpis.is_empty() {
            None
        } else {
            Some(KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis))
        };

        let sessions: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                let kpi_scores = kpi_scorer.as_ref().map(|ks| {
                    let classified = ks.classify(&r.record);
                    let mut scores: Vec<serde_json::Value> = classified
                        .kpi_scores
                        .iter()
                        .map(|k| serde_json::json!({ "kpi_id": k.kpi_id, "score": k.score }))
                        .collect();
                    scores.sort_by(|a, b| {
                        b["score"]
                            .as_f64()
                            .partial_cmp(&a["score"].as_f64())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    scores
                });
                let panelist_scores: Vec<serde_json::Value> = r
                    .scores
                    .iter()
                    .map(|s| serde_json::json!({ "role": s.role, "score": s.score }))
                    .collect();
                serde_json::json!({
                    "id": r.record.id,
                    "timestamp": r.record.timestamp.to_rfc3339(),
                    "title": r.record.title,
                    "source": r.record.source,
                    "consensus": r.consensus,
                    "dissent": r.dissent,
                    "high_dissent": r.dissent > DISSENT_THRESHOLD,
                    "champion": r.champion,
                    "skeptic": r.skeptic,
                    "panelist_scores": panelist_scores,
                    "kpi_scores": kpi_scores,
                })
            })
            .collect();

        let count = sessions.len() as f32;
        let avg_consensus: f32 = results.iter().map(|r| r.consensus).sum::<f32>() / count;
        let avg_dissent: f32 = results.iter().map(|r| r.dissent).sum::<f32>() / count;
        let high_dissent_count = results
            .iter()
            .filter(|r| r.dissent > DISSENT_THRESHOLD)
            .count();

        let output = serde_json::json!({
            "since": params.since,
            "session_count": sessions.len(),
            "period_avg_consensus": avg_consensus,
            "period_avg_dissent": avg_dissent,
            "high_dissent_count": high_dissent_count,
            "sessions": sessions,
        });

        Ok(text_result(
            serde_json::to_string_pretty(&output).unwrap_or_default(),
        ))
    }

    #[tool(
        description = "Categorized effort breakdown by activity type. Returns record count, effort share, and KPI alignment scores per category."
    )]
    fn meridian_analyze(
        &self,
        Parameters(params): Parameters<AnalyzeParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = MeridianConfig::load_or_default(&self.config_path);
        let taxonomy = config.taxonomy();

        if taxonomy.categories.is_empty() {
            return Ok(text_result(
                "No categories defined in meridian.toml.".to_string(),
            ));
        }

        let records = load_records(
            params.source.as_deref(),
            params.path.as_deref().map(Path::new),
            &config,
        )
        .map_err(mcp_err)?;

        if records.is_empty() {
            return Ok(text_result("No records found.".to_string()));
        }

        let scorer = KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis);
        let classified = scorer.classify_batch(&records);
        let report = summary::summarise(&classified);

        let categories: Vec<serde_json::Value> = report
            .by_category
            .iter()
            .map(|cat| {
                serde_json::json!({
                    "category_id": cat.category_id,
                    "record_count": cat.record_count,
                    "total_effort": cat.total_effort,
                    "effort_pct": cat.effort_pct,
                    "kpi_alignment": cat.kpi_alignment,
                })
            })
            .collect();

        let output = serde_json::json!({
            "total_records": report.total_records,
            "total_effort": report.total_effort,
            "categories": categories,
        });

        Ok(text_result(
            serde_json::to_string_pretty(&output).unwrap_or_default(),
        ))
    }

    #[tool(
        description = "List configured KPIs from meridian.toml including weights, descriptions, and measurable metrics."
    )]
    fn meridian_kpis(
        &self,
        Parameters(_params): Parameters<KpisParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = MeridianConfig::load_or_default(&self.config_path);
        let taxonomy = config.taxonomy();

        if taxonomy.kpis.is_empty() {
            return Ok(text_result(
                "No KPIs defined. Run `meridian init` or `meridian pm init` to create a config."
                    .to_string(),
            ));
        }

        let kpis: Vec<serde_json::Value> = taxonomy
            .kpis
            .iter()
            .map(|k| {
                let metrics: Vec<serde_json::Value> = k
                    .metrics
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "label": m.label,
                            "unit": m.unit,
                            "target": m.target,
                            "source": m.source,
                            "frequency": m.frequency,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "id": k.id,
                    "label": k.label,
                    "description": k.description,
                    "weight": k.weight,
                    "metrics": metrics,
                })
            })
            .collect();

        let output = serde_json::json!({ "kpis": kpis });
        Ok(text_result(
            serde_json::to_string_pretty(&output).unwrap_or_default(),
        ))
    }

    #[tool(
        description = "Find category×KPI pairs below the alignment threshold. Returns low-scoring pairs sorted by score. Set ANTHROPIC_API_KEY or install the claude CLI, then run `meridian suggest` for AI recommendations."
    )]
    fn meridian_suggest(
        &self,
        Parameters(params): Parameters<SuggestParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = MeridianConfig::load_or_default(&self.config_path);
        let taxonomy = config.taxonomy();

        if taxonomy.kpis.is_empty() {
            return Ok(text_result(
                "No KPIs defined. Run `meridian init` or `meridian pm init` first.".to_string(),
            ));
        }

        let records = load_records(
            params.source.as_deref(),
            params.path.as_deref().map(Path::new),
            &config,
        )
        .map_err(mcp_err)?;

        if records.is_empty() {
            return Ok(text_result("No records found.".to_string()));
        }

        let scorer = KeywordScorer::new(&taxonomy.categories, &taxonomy.kpis);
        let classified = scorer.classify_batch(&records);
        let report = summary::summarise(&classified);

        let threshold = params.threshold.unwrap_or(config.suggest.threshold);
        let kpi_labels: HashMap<String, String> = taxonomy
            .kpis
            .iter()
            .map(|k| (k.id.clone(), k.label.clone()))
            .collect();

        let low_pairs =
            suggest_lib::find_low_alignment_pairs(&report.by_category, threshold, &kpi_labels);

        if low_pairs.is_empty() {
            return Ok(text_result(format!(
                "All category×KPI pairs are above the {:.0}% threshold.",
                threshold * 100.0
            )));
        }

        let pairs: Vec<serde_json::Value> = low_pairs
            .iter()
            .map(|(cat_id, kpi_label, score)| {
                serde_json::json!({
                    "category_id": cat_id,
                    "kpi_label": kpi_label,
                    "alignment_score": score,
                })
            })
            .collect();

        let output = serde_json::json!({
            "threshold": threshold,
            "low_alignment_count": pairs.len(),
            "low_alignment_pairs": pairs,
            "hint": "Run `meridian suggest` with ANTHROPIC_API_KEY for AI-generated recommendations.",
        });

        Ok(text_result(
            serde_json::to_string_pretty(&output).unwrap_or_default(),
        ))
    }
}

#[tool_handler]
impl ServerHandler for MeridianMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "meridian".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("Meridian — Token Analytics & KPI Alignment".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Meridian analytics MCP server. \
                 meridian_report: expert panel KPI alignment by session. \
                 meridian_analyze: categorized effort breakdown. \
                 meridian_kpis: list configured KPIs and metrics. \
                 meridian_suggest: find low-alignment category×KPI pairs."
                    .into(),
            ),
        }
    }
}

pub fn run(_args: ServeArgs, config_path: &Path) -> Result<()> {
    let server = MeridianMcpServer::new(config_path.to_path_buf());
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let transport = rmcp::transport::stdio();
        let handle = server
            .serve(transport)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
        let _ = handle.waiting().await;
        Ok::<(), anyhow::Error>(())
    })
}

fn text_result(s: String) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(s)],
        is_error: None,
        meta: None,
        structured_content: None,
    }
}

fn mcp_err(e: anyhow::Error) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn load_records(
    source: Option<&str>,
    path: Option<&Path>,
    config: &MeridianConfig,
) -> anyhow::Result<Vec<UsageRecord>> {
    let source_type = source.unwrap_or("auto");
    match source_type {
        "ta" => {
            let root = path
                .or_else(|| config.source.ta_project_root.as_deref().map(Path::new))
                .unwrap_or(Path::new("."));
            let vel_path = root.join(".ta").join("velocity-history.jsonl");
            if vel_path.exists() {
                TaSource::load_velocity(&vel_path)
            } else {
                TaSource::discover(root)
                    .ok_or_else(|| anyhow::anyhow!("No .ta/velocity-history.jsonl found"))
                    .and_then(|p| TaSource::load_velocity(&p))
            }
        }
        "jsonl" => {
            let p = path
                .or_else(|| config.source.jsonl.as_deref().map(Path::new))
                .ok_or_else(|| {
                    anyhow::anyhow!("Specify path or set source.jsonl in meridian.toml")
                })?;
            generic::load_jsonl(p)
        }
        "claude-code" => {
            let dir = path
                .map(PathBuf::from)
                .or_else(|| config.source.claude_code_dir.as_deref().map(PathBuf::from))
                .unwrap_or_else(claude_code::default_projects_dir);
            claude_code::load_all(&dir)
        }
        _ => {
            if let Some(vel) = TaSource::discover(Path::new(".")) {
                TaSource::load_velocity(&vel)
            } else if let Some(p) = config.source.jsonl.as_deref() {
                generic::load_jsonl(Path::new(p))
            } else {
                let cc_dir = config
                    .source
                    .claude_code_dir
                    .as_deref()
                    .map(PathBuf::from)
                    .unwrap_or_else(claude_code::default_projects_dir);
                if cc_dir.exists() {
                    claude_code::load_all(&cc_dir)
                } else {
                    anyhow::bail!(
                        "No data source found. Use source=ta, claude-code, or jsonl with a path."
                    )
                }
            }
        }
    }
}
