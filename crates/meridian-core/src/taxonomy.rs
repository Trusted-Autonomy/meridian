use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kpi {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

/// L2 abstract sub-type, cross-domain comparable.
/// domain_label overrides the display label for a specific vertical
/// (e.g. "Gameplay Mechanics" for gamedev instead of "Core Feature Development").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subcategory {
    pub id: String,
    pub parent_id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub domain_label: Option<String>,
}

impl Subcategory {
    pub fn display_label(&self) -> &str {
        self.domain_label.as_deref().unwrap_or(&self.label)
    }
}

#[derive(Debug, Clone)]
pub struct Taxonomy {
    pub categories: Vec<Category>,
    pub kpis: Vec<Kpi>,
    pub subcategories: Vec<Subcategory>,
}

impl Taxonomy {
    pub fn builtin() -> Self {
        let categories = vec![
            Category {
                id: "code".into(),
                label: "Code Implementation".into(),
                description: "Writing, debugging, refactoring, reviewing code, tests, CI/CD".into(),
                keywords: vec![
                    "implement".into(),
                    "fix".into(),
                    "refactor".into(),
                    "test".into(),
                    "build".into(),
                    "compile".into(),
                    "debug".into(),
                    "feature".into(),
                    "api".into(),
                    "rust".into(),
                    "function".into(),
                    "module".into(),
                    "crate".into(),
                ],
            },
            Category {
                id: "pm".into(),
                label: "Project Management".into(),
                description: "Planning, roadmaps, sprints, tracking, retrospectives, milestones"
                    .into(),
                keywords: vec![
                    "plan".into(),
                    "phase".into(),
                    "milestone".into(),
                    "roadmap".into(),
                    "sprint".into(),
                    "backlog".into(),
                    "estimate".into(),
                    "schedule".into(),
                    "status".into(),
                    "progress".into(),
                ],
            },
            Category {
                id: "docs".into(),
                label: "Documentation".into(),
                description: "Writing internal docs, usage guides, changelogs, READMEs, onboarding"
                    .into(),
                keywords: vec![
                    "doc".into(),
                    "document".into(),
                    "readme".into(),
                    "changelog".into(),
                    "usage".into(),
                    "guide".into(),
                    "onboard".into(),
                    "write".into(),
                    "explain".into(),
                ],
            },
            Category {
                id: "security".into(),
                label: "Security & Compliance".into(),
                description: "Security review, hardening, audit, compliance, access control".into(),
                keywords: vec![
                    "security".into(),
                    "audit".into(),
                    "hardening".into(),
                    "compliance".into(),
                    "permission".into(),
                    "auth".into(),
                    "vulnerability".into(),
                    "cve".into(),
                    "policy".into(),
                ],
            },
            Category {
                id: "ops".into(),
                label: "Operations & Infrastructure".into(),
                description: "Deployment, CI/CD pipelines, release management, monitoring, devops"
                    .into(),
                keywords: vec![
                    "deploy".into(),
                    "release".into(),
                    "pipeline".into(),
                    "ci".into(),
                    "cd".into(),
                    "workflow".into(),
                    "monitor".into(),
                    "infra".into(),
                    "docker".into(),
                    "nix".into(),
                ],
            },
            Category {
                id: "finance".into(),
                label: "Finance & Reporting".into(),
                description: "Budget analysis, cost tracking, financial modeling, reporting".into(),
                keywords: vec![
                    "budget".into(),
                    "cost".into(),
                    "finance".into(),
                    "revenue".into(),
                    "spend".into(),
                    "token".into(),
                    "billing".into(),
                    "report".into(),
                    "kpi".into(),
                    "metric".into(),
                ],
            },
            Category {
                id: "sales".into(),
                label: "Sales & Business Development".into(),
                description: "Sales support, proposals, CRM, business development, outreach".into(),
                keywords: vec![
                    "sales".into(),
                    "proposal".into(),
                    "customer".into(),
                    "client".into(),
                    "deal".into(),
                    "prospect".into(),
                    "crm".into(),
                    "pitch".into(),
                    "contract".into(),
                ],
            },
            Category {
                id: "marketing".into(),
                label: "Marketing & Content".into(),
                description: "Content creation, marketing copy, campaigns, social, brand".into(),
                keywords: vec![
                    "marketing".into(),
                    "content".into(),
                    "campaign".into(),
                    "brand".into(),
                    "copy".into(),
                    "social".into(),
                    "blog".into(),
                    "announcement".into(),
                ],
            },
            Category {
                id: "product".into(),
                label: "Product Management".into(),
                description: "Product discovery, user research, requirements, PRDs, UX decisions"
                    .into(),
                keywords: vec![
                    "product".into(),
                    "user".into(),
                    "ux".into(),
                    "requirement".into(),
                    "prd".into(),
                    "discovery".into(),
                    "feedback".into(),
                    "design".into(),
                ],
            },
            Category {
                id: "research".into(),
                label: "Research & Exploration".into(),
                description: "Technical research, prototyping, POCs, investigation, analysis"
                    .into(),
                keywords: vec![
                    "research".into(),
                    "explore".into(),
                    "investigate".into(),
                    "poc".into(),
                    "prototype".into(),
                    "analysis".into(),
                    "benchmark".into(),
                    "evaluate".into(),
                ],
            },
        ];

        let subcategories = builtin_subcategories();

        Taxonomy {
            categories,
            kpis: vec![],
            subcategories,
        }
    }

    pub fn subcategories_for(&self, parent_id: &str) -> Vec<&Subcategory> {
        self.subcategories
            .iter()
            .filter(|s| s.parent_id == parent_id)
            .collect()
    }
}

fn builtin_subcategories() -> Vec<Subcategory> {
    vec![
        // code
        Subcategory {
            id: "code.core_feature".into(),
            parent_id: "code".into(),
            label: "Core Feature Development".into(),
            description: "Implementing primary product functionality, business logic, domain rules"
                .into(),
            keywords: vec![
                "feature".into(),
                "implement".into(),
                "logic".into(),
                "engine".into(),
                "system".into(),
                "core".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.data_model".into(),
            parent_id: "code".into(),
            label: "Data Modeling".into(),
            description: "Schema design, entity definitions, migrations, serialization, storage"
                .into(),
            keywords: vec![
                "schema".into(),
                "model".into(),
                "migration".into(),
                "database".into(),
                "struct".into(),
                "entity".into(),
                "table".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.integration".into(),
            parent_id: "code".into(),
            label: "Integration & Connectors".into(),
            description: "External API integration, plugins, adapters, SDKs, webhooks, OAuth"
                .into(),
            keywords: vec![
                "integration".into(),
                "connector".into(),
                "adapter".into(),
                "webhook".into(),
                "oauth".into(),
                "plugin".into(),
                "sdk".into(),
                "client".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.ui".into(),
            parent_id: "code".into(),
            label: "UI / Frontend".into(),
            description: "UI components, views, forms, styles, client-side logic, rendering".into(),
            keywords: vec![
                "ui".into(),
                "frontend".into(),
                "component".into(),
                "view".into(),
                "render".into(),
                "form".into(),
                "style".into(),
                "css".into(),
                "react".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.testing".into(),
            parent_id: "code".into(),
            label: "Testing & QA".into(),
            description:
                "Unit tests, integration tests, test fixtures, assertions, CI verification".into(),
            keywords: vec![
                "test".into(),
                "spec".into(),
                "assert".into(),
                "mock".into(),
                "fixture".into(),
                "verify".into(),
                "coverage".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.performance".into(),
            parent_id: "code".into(),
            label: "Performance & Reliability".into(),
            description: "Optimization, caching, profiling, resilience, error handling, stability"
                .into(),
            keywords: vec![
                "performance".into(),
                "optimize".into(),
                "cache".into(),
                "latency".into(),
                "throughput".into(),
                "reliability".into(),
                "resilience".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "code.refactor".into(),
            parent_id: "code".into(),
            label: "Refactoring & Tech Debt".into(),
            description: "Cleanup, restructuring, abstraction, modularization, dead code removal"
                .into(),
            keywords: vec![
                "refactor".into(),
                "cleanup".into(),
                "restructure".into(),
                "abstract".into(),
                "modular".into(),
                "dead".into(),
                "simplify".into(),
            ],
            domain_label: None,
        },
        // pm
        Subcategory {
            id: "pm.planning".into(),
            parent_id: "pm".into(),
            label: "Sprint / Release Planning".into(),
            description:
                "Sprint planning, milestone setting, estimation, backlog grooming, roadmap".into(),
            keywords: vec![
                "sprint".into(),
                "plan".into(),
                "milestone".into(),
                "estimate".into(),
                "backlog".into(),
                "roadmap".into(),
                "prioritize".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "pm.tracking".into(),
            parent_id: "pm".into(),
            label: "Progress Tracking".into(),
            description: "Status updates, velocity tracking, burndown, retrospective, OKR review"
                .into(),
            keywords: vec![
                "status".into(),
                "progress".into(),
                "velocity".into(),
                "retrospective".into(),
                "okr".into(),
                "burndown".into(),
                "review".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "pm.coordination".into(),
            parent_id: "pm".into(),
            label: "Team Coordination".into(),
            description: "Cross-team dependencies, blockers, handoffs, stakeholder communication"
                .into(),
            keywords: vec![
                "coordinate".into(),
                "handoff".into(),
                "stakeholder".into(),
                "dependency".into(),
                "blocker".into(),
                "sync".into(),
                "meeting".into(),
            ],
            domain_label: None,
        },
        // docs
        Subcategory {
            id: "docs.user_guide".into(),
            parent_id: "docs".into(),
            label: "User / API Guide".into(),
            description: "Usage documentation, tutorials, API reference, how-to guides, onboarding"
                .into(),
            keywords: vec![
                "usage".into(),
                "tutorial".into(),
                "guide".into(),
                "api".into(),
                "reference".into(),
                "example".into(),
                "quickstart".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "docs.internal".into(),
            parent_id: "docs".into(),
            label: "Internal Knowledge Base".into(),
            description:
                "Design docs, architecture notes, runbooks, ADRs, post-mortems, team wikis".into(),
            keywords: vec![
                "adr".into(),
                "runbook".into(),
                "design".into(),
                "architecture".into(),
                "postmortem".into(),
                "internal".into(),
                "wiki".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "docs.changelog".into(),
            parent_id: "docs".into(),
            label: "Release Notes & Changelog".into(),
            description: "Changelogs, release announcements, upgrade guides, version notes".into(),
            keywords: vec![
                "changelog".into(),
                "release".into(),
                "notes".into(),
                "announcement".into(),
                "upgrade".into(),
                "version".into(),
            ],
            domain_label: None,
        },
        // security
        Subcategory {
            id: "security.review".into(),
            parent_id: "security".into(),
            label: "Security Review".into(),
            description:
                "Code security review, threat modeling, vulnerability assessment, pen test".into(),
            keywords: vec![
                "review".into(),
                "threat".into(),
                "vulnerability".into(),
                "pentest".into(),
                "assessment".into(),
                "cve".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "security.hardening".into(),
            parent_id: "security".into(),
            label: "System Hardening".into(),
            description: "Access control, secrets management, permissions tightening, encryption"
                .into(),
            keywords: vec![
                "hardening".into(),
                "secrets".into(),
                "encryption".into(),
                "permission".into(),
                "access".into(),
                "control".into(),
                "token".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "security.compliance".into(),
            parent_id: "security".into(),
            label: "Compliance & Audit".into(),
            description: "Regulatory compliance, audit logging, policy enforcement, certifications"
                .into(),
            keywords: vec![
                "compliance".into(),
                "audit".into(),
                "regulation".into(),
                "policy".into(),
                "gdpr".into(),
                "soc2".into(),
                "certification".into(),
            ],
            domain_label: None,
        },
        // ops
        Subcategory {
            id: "ops.deployment".into(),
            parent_id: "ops".into(),
            label: "Deployment & Release".into(),
            description: "Deployment pipelines, release automation, rollout management, versioning"
                .into(),
            keywords: vec![
                "deploy".into(),
                "release".into(),
                "rollout".into(),
                "version".into(),
                "tag".into(),
                "publish".into(),
                "ship".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "ops.ci_cd".into(),
            parent_id: "ops".into(),
            label: "CI/CD Pipelines".into(),
            description: "Test automation, build pipelines, artifact publishing, GH Actions".into(),
            keywords: vec![
                "ci".into(),
                "cd".into(),
                "pipeline".into(),
                "github".into(),
                "actions".into(),
                "build".into(),
                "workflow".into(),
                "artifact".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "ops.monitoring".into(),
            parent_id: "ops".into(),
            label: "Monitoring & Observability".into(),
            description: "Metrics collection, alerting, log aggregation, tracing, dashboards"
                .into(),
            keywords: vec![
                "monitor".into(),
                "alert".into(),
                "log".into(),
                "trace".into(),
                "metric".into(),
                "dashboard".into(),
                "observability".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "ops.infra".into(),
            parent_id: "ops".into(),
            label: "Infrastructure".into(),
            description:
                "Server provisioning, config management, Nix/Docker/k8s, networking, storage".into(),
            keywords: vec![
                "infra".into(),
                "nix".into(),
                "docker".into(),
                "kubernetes".into(),
                "server".into(),
                "provision".into(),
                "network".into(),
            ],
            domain_label: None,
        },
        // finance
        Subcategory {
            id: "finance.analysis".into(),
            parent_id: "finance".into(),
            label: "Financial Analysis".into(),
            description: "Budget modeling, cost analysis, ROI calculation, spend breakdown".into(),
            keywords: vec![
                "cost".into(),
                "budget".into(),
                "roi".into(),
                "analysis".into(),
                "spend".into(),
                "forecast".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "finance.reporting".into(),
            parent_id: "finance".into(),
            label: "Financial Reporting".into(),
            description: "Financial dashboards, budget vs actuals, spend reports, KPI metrics"
                .into(),
            keywords: vec![
                "report".into(),
                "dashboard".into(),
                "actuals".into(),
                "kpi".into(),
                "metric".into(),
                "revenue".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "finance.forecasting".into(),
            parent_id: "finance".into(),
            label: "Forecasting & Planning".into(),
            description: "Financial projections, capacity planning, headcount modeling, scenario"
                .into(),
            keywords: vec![
                "forecast".into(),
                "projection".into(),
                "planning".into(),
                "headcount".into(),
                "scenario".into(),
                "capacity".into(),
            ],
            domain_label: None,
        },
        // sales
        Subcategory {
            id: "sales.proposal".into(),
            parent_id: "sales".into(),
            label: "Sales Proposals".into(),
            description: "Writing proposals, RFP responses, pitch decks, statements of work".into(),
            keywords: vec![
                "proposal".into(),
                "rfp".into(),
                "pitch".into(),
                "deck".into(),
                "statement".into(),
                "work".into(),
                "sow".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "sales.enablement".into(),
            parent_id: "sales".into(),
            label: "Sales Enablement".into(),
            description:
                "Sales docs, competitive analysis, product positioning, objection handling".into(),
            keywords: vec![
                "enablement".into(),
                "competitive".into(),
                "positioning".into(),
                "objection".into(),
                "battlecard".into(),
                "playbook".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "sales.crm".into(),
            parent_id: "sales".into(),
            label: "CRM & Pipeline".into(),
            description: "CRM updates, pipeline management, deal tracking, account notes".into(),
            keywords: vec![
                "crm".into(),
                "pipeline".into(),
                "deal".into(),
                "account".into(),
                "opportunity".into(),
                "prospect".into(),
            ],
            domain_label: None,
        },
        // marketing
        Subcategory {
            id: "marketing.content".into(),
            parent_id: "marketing".into(),
            label: "Content Creation".into(),
            description: "Blog posts, landing pages, social content, case studies, newsletters"
                .into(),
            keywords: vec![
                "blog".into(),
                "post".into(),
                "landing".into(),
                "social".into(),
                "newsletter".into(),
                "case".into(),
                "study".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "marketing.campaign".into(),
            parent_id: "marketing".into(),
            label: "Campaign Management".into(),
            description: "Campaign planning, ad copy, A/B testing, email sequences, analytics"
                .into(),
            keywords: vec![
                "campaign".into(),
                "ad".into(),
                "email".into(),
                "ab".into(),
                "test".into(),
                "conversion".into(),
                "funnel".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "marketing.brand".into(),
            parent_id: "marketing".into(),
            label: "Brand & Design".into(),
            description: "Brand guidelines, visual assets, messaging frameworks, tone of voice"
                .into(),
            keywords: vec![
                "brand".into(),
                "design".into(),
                "visual".into(),
                "asset".into(),
                "messaging".into(),
                "tone".into(),
                "style".into(),
            ],
            domain_label: None,
        },
        // product
        Subcategory {
            id: "product.discovery".into(),
            parent_id: "product".into(),
            label: "Product Discovery".into(),
            description: "User research, problem definition, opportunity mapping, hypothesis"
                .into(),
            keywords: vec![
                "discovery".into(),
                "research".into(),
                "user".into(),
                "problem".into(),
                "opportunity".into(),
                "hypothesis".into(),
                "interview".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "product.requirements".into(),
            parent_id: "product".into(),
            label: "Requirements & PRDs".into(),
            description:
                "Feature specs, user stories, acceptance criteria, PRDs, product requirements"
                    .into(),
            keywords: vec![
                "requirement".into(),
                "prd".into(),
                "story".into(),
                "criteria".into(),
                "spec".into(),
                "acceptance".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "product.ux_design".into(),
            parent_id: "product".into(),
            label: "UX / Experience Design".into(),
            description: "User flows, wireframes, interaction design, prototypes, usability".into(),
            keywords: vec![
                "ux".into(),
                "wireframe".into(),
                "flow".into(),
                "interaction".into(),
                "prototype".into(),
                "usability".into(),
                "figma".into(),
            ],
            domain_label: None,
        },
        // research
        Subcategory {
            id: "research.technical".into(),
            parent_id: "research".into(),
            label: "Technical Research".into(),
            description: "Technology evaluation, architecture investigation, benchmarking, spikes"
                .into(),
            keywords: vec![
                "evaluate".into(),
                "benchmark".into(),
                "spike".into(),
                "investigate".into(),
                "compare".into(),
                "technology".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "research.market".into(),
            parent_id: "research".into(),
            label: "Market Research".into(),
            description: "Competitive analysis, industry trends, customer insights, landscape"
                .into(),
            keywords: vec![
                "competitive".into(),
                "market".into(),
                "industry".into(),
                "trend".into(),
                "landscape".into(),
                "competitor".into(),
            ],
            domain_label: None,
        },
        Subcategory {
            id: "research.poc".into(),
            parent_id: "research".into(),
            label: "Prototyping & POC".into(),
            description: "Proof-of-concept, experiment design, hypothesis testing, exploration"
                .into(),
            keywords: vec![
                "poc".into(),
                "prototype".into(),
                "experiment".into(),
                "explore".into(),
                "proof".into(),
                "concept".into(),
            ],
            domain_label: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_ten_categories() {
        let t = Taxonomy::builtin();
        assert_eq!(t.categories.len(), 10);
    }

    #[test]
    fn builtin_has_subcategories_for_every_category() {
        let t = Taxonomy::builtin();
        for cat in &t.categories {
            let subs = t.subcategories_for(&cat.id);
            assert!(!subs.is_empty(), "category {} has no subcategories", cat.id);
        }
    }

    #[test]
    fn subcategory_parent_ids_are_valid() {
        let t = Taxonomy::builtin();
        let cat_ids: std::collections::HashSet<&str> =
            t.categories.iter().map(|c| c.id.as_str()).collect();
        for sub in &t.subcategories {
            assert!(
                cat_ids.contains(sub.parent_id.as_str()),
                "subcategory {} has unknown parent {}",
                sub.id,
                sub.parent_id
            );
        }
    }

    #[test]
    fn display_label_uses_domain_override() {
        let mut sub = Subcategory {
            id: "code.core_feature".into(),
            parent_id: "code".into(),
            label: "Core Feature Development".into(),
            description: "".into(),
            keywords: vec![],
            domain_label: None,
        };
        assert_eq!(sub.display_label(), "Core Feature Development");
        sub.domain_label = Some("Gameplay Mechanics".into());
        assert_eq!(sub.display_label(), "Gameplay Mechanics");
    }
}
