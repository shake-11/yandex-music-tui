//! the user model.

use serde::{Deserialize, Serialize};

/// a yandex music user, for example a playlist owner.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct User {
    /// user uid.
    pub uid: Option<i64>,
    /// login.
    pub login: Option<String>,
    /// account name.
    pub name: Option<String>,
    /// display name.
    pub display_name: Option<String>,
    /// full name.
    pub full_name: Option<String>,
    /// reported sex.
    pub sex: Option<String>,
    /// whether the user is verified.
    pub verified: Option<bool>,
    /// region codes.
    pub regions: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case_keys_map_onto_snake_case_fields() {
        let user: User =
            serde_json::from_str(r#"{"uid":1,"displayName":"Ann","fullName":"Ann B"}"#).unwrap();
        assert_eq!(user.uid, Some(1));
        assert_eq!(user.display_name.as_deref(), Some("Ann"));
        assert_eq!(user.full_name.as_deref(), Some("Ann B"));
    }

    #[test]
    fn an_empty_object_deserializes() {
        let user: User = serde_json::from_str("{}").unwrap();
        assert!(user.uid.is_none());
        assert!(user.regions.is_empty());
    }
}
