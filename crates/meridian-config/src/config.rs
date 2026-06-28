use anyhow::{Context, Result};
use meridian_core::panel::default_panel;
use meridian_core::taxonomy::{Category, Kpi, Taxonomy};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use meridian_core::panel::PanelMember;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MeridianConfig {
    #[serde(default)]
    pub categories: Vec<CategoryConfig>,
    #[serde(default)]
    pub kpis: Vec<KpiConfig>,
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub report: ReportConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub suggest: SuggestConfig,
    /// Expert panel for `meridian report`. Falls back to built-in 4-role panel when empty.
    #[serde(default)]
    pub panel: Vec<PanelMember>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// "keyword" (default, zero cost) or "voyage" (semantic, requires VOYAGE_API_KEY)
    #[serde(default = "default_embedding_backend")]
    pub backend: String,
    /// Voyage model name (default: voyage-3-lite)
    pub model: Option<String>,
    /// Explicit API key (prefer VOYAGE_API_KEY env var instead)
    pub api_key: Option<String>,
}

fn default_embedding_backend() -> String {
    "keyword".to_string()
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend: default_embedding_backend(),
            model: None,
            api_key: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestConfig {
    /// KPI alignment below this triggers suggestions (0.0–1.0)
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    /// Max record titles to include as context per suggestion
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,
    /// Claude model to use for suggestions
    #[serde(default = "default_suggest_model")]
    pub model: String,
    /// Explicit API key (prefer ANTHROPIC_API_KEY env var instead)
    pub api_key: Option<String>,
}

fn default_threshold() -> f32 {
    0.25
}
fn default_sample_size() -> usize {
    5
}
fn default_suggest_model() -> String {
    "claude-haiku-4-5-20251001".to_string()
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self {
            threshold: default_threshold(),
            sample_size: default_sample_size(),
            model: default_suggest_model(),
            api_key: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KpiConfig {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SourceConfig {
    pub ta_project_root: Option<String>,
    pub jsonl: Option<String>,
    /// Path to a Claude Code projects directory (default: ~/.claude/projects)
    pub claude_code_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ReportConfig {
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "table".to_string()
}

impl MeridianConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            Self::load(path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Returns the user-configured panel if non-empty, otherwise the built-in 4-role panel.
    pub fn panel_effective(&self) -> Vec<PanelMember> {
        if self.panel.is_empty() {
            default_panel()
        } else {
            self.panel.clone()
        }
    }

    /// Build a Taxonomy: built-in categories unless overridden; user KPIs merged on top.
    pub fn taxonomy(&self) -> Taxonomy {
        let mut tax = Taxonomy::builtin();

        if !self.categories.is_empty() {
            tax.categories = self
                .categories
                .iter()
                .map(|c| Category {
                    id: c.id.clone(),
                    label: c.label.clone(),
                    description: c.description.clone(),
                    keywords: c.keywords.clone(),
                })
                .collect();
        }

        tax.kpis = self
            .kpis
            .iter()
            .map(|k| Kpi {
                id: k.id.clone(),
                label: k.label.clone(),
                description: k.description.clone(),
                weight: k.weight,
            })
            .collect();

        tax
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_builtin_categories() {
        let cfg = MeridianConfig::default();
        let tax = cfg.taxonomy();
        assert!(!tax.categories.is_empty());
        assert!(tax.categories.iter().any(|c| c.id == "code"));
    }

    #[test]
    fn kpis_from_config() {
        let toml = r#"
[[kpis]]
id = "eng_velocity"
label = "Engineering Velocity"
description = "ship quality software faster"
weight = 1.0
"#;
        let cfg: MeridianConfig = toml::from_str(toml).unwrap();
        let tax = cfg.taxonomy();
        assert_eq!(tax.kpis.len(), 1);
        assert_eq!(tax.kpis[0].id, "eng_velocity");
    }
}
