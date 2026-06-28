use crate::scorer::{cosine, tf_vector, tokenise};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelMember {
    pub role: String,
    /// Free-text description of what this role cares about — scored via TF-IDF.
    pub charter: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

/// Built-in executive panel used when no `[[panel]]` is configured.
pub fn default_panel() -> Vec<PanelMember> {
    vec![
        PanelMember {
            role: "ceo".to_string(),
            charter: "Revenue growth, market position, strategic alignment, return on AI investment, business outcomes, competitive advantage".to_string(),
            weight: 1.0,
        },
        PanelMember {
            role: "cto".to_string(),
            charter: "Technical debt reduction, system reliability, security posture, architectural integrity, engineering standards, platform scalability".to_string(),
            weight: 1.0,
        },
        PanelMember {
            role: "head_of_product".to_string(),
            charter: "User value delivery, feature velocity, discovery quality, roadmap coherence, product-market fit, customer satisfaction".to_string(),
            weight: 1.0,
        },
        PanelMember {
            role: "head_of_engineering".to_string(),
            charter: "Code quality, test coverage, delivery predictability, team unblocking, developer experience, operational excellence".to_string(),
            weight: 0.8,
        },
    ]
}

/// Pre-compute a TF-IDF vector from a panelist's charter text.
pub fn charter_vector(member: &PanelMember) -> HashMap<String, f32> {
    tf_vector(&tokenise(&member.charter))
}

/// Score a session title against a pre-computed charter vector.
pub fn score_title(title: &str, charter_vec: &HashMap<String, f32>) -> f32 {
    cosine(&tf_vector(&tokenise(title)), charter_vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_panel_has_four_members() {
        let panel = default_panel();
        assert_eq!(panel.len(), 4);
        assert!(panel.iter().any(|m| m.role == "ceo"));
        assert!(panel.iter().any(|m| m.role == "cto"));
        assert!(panel.iter().any(|m| m.role == "head_of_product"));
        assert!(panel.iter().any(|m| m.role == "head_of_engineering"));
    }

    #[test]
    fn charter_vector_non_empty() {
        let panel = default_panel();
        let vec = charter_vector(&panel[0]);
        assert!(!vec.is_empty());
    }

    #[test]
    fn score_title_security_cto_higher_than_ceo() {
        let panel = default_panel();
        let ceo = charter_vector(panel.iter().find(|m| m.role == "ceo").unwrap());
        let cto = charter_vector(panel.iter().find(|m| m.role == "cto").unwrap());
        let ceo_score = score_title("fix critical security vulnerability in auth system", &ceo);
        let cto_score = score_title("fix critical security vulnerability in auth system", &cto);
        assert!(
            cto_score > ceo_score,
            "CTO ({cto_score:.3}) should score higher than CEO ({ceo_score:.3}) for security work"
        );
    }
}
