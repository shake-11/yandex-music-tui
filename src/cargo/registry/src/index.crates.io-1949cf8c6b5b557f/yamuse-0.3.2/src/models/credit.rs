//! production credits.

use serde::{Deserialize, Serialize};

/// a single production credit: a role and its value.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Credit {
    /// credit title — the role, for example "producer".
    pub title: Option<String>,
    /// credit value — the person or entity.
    pub value: Option<String>,
}

/// the collection of production credits for an entity.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Credits {
    /// the individual credits.
    pub credits: Vec<Credit>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_deserialize_as_role_value_pairs() {
        let credits: Credits =
            serde_json::from_str(r#"{"credits":[{"title":"Producer","value":"X"}]}"#).unwrap();
        assert_eq!(credits.credits[0].title.as_deref(), Some("Producer"));
        assert_eq!(credits.credits[0].value.as_deref(), Some("X"));
    }

    #[test]
    fn credit_models_survive_an_empty_object() {
        serde_json::from_str::<Credit>("{}").unwrap();
        serde_json::from_str::<Credits>("{}").unwrap();
    }
}
