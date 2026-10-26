//! legal disclaimers attached to catalogue entities.

use serde::{Deserialize, Serialize};

/// a "foreign agent" notice attached to a disclaimer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ForeignAgent {
    /// reason for the notice; `policy` is the known value.
    pub reason: Option<String>,
    /// notice title.
    pub title: Option<String>,
}

/// a legal disclaimer attached to a track, clip, album or artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Disclaimer {
    /// the foreign-agent notice, when present.
    pub foreign_agent: Option<ForeignAgent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_foreign_agent_notice_deserializes() {
        let disclaimer: Disclaimer =
            serde_json::from_str(r#"{"foreignAgent":{"reason":"policy","title":"notice"}}"#)
                .unwrap();
        let agent = disclaimer.foreign_agent.unwrap();
        assert_eq!(agent.reason.as_deref(), Some("policy"));
        assert_eq!(agent.title.as_deref(), Some("notice"));
    }

    #[test]
    fn disclaimer_models_survive_an_empty_object() {
        let empty: Disclaimer = serde_json::from_str("{}").unwrap();
        assert!(empty.foreign_agent.is_none());
        serde_json::from_str::<ForeignAgent>("{}").unwrap();
    }
}
