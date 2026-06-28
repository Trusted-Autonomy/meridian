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

#[derive(Debug, Clone)]
pub struct Taxonomy {
    pub categories: Vec<Category>,
    pub kpis: Vec<Kpi>,
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

        Taxonomy {
            categories,
            kpis: vec![],
        }
    }
}
