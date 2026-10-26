//! oauth device-flow payloads.
//!
//! these two are the only models the api answers in snake_case, because they come
//! from the oauth service rather than the music api.

use serde::{Deserialize, Serialize};

/// the device and user code pair issued at the start of the device flow.
///
/// show [`user_code`](Self::user_code) to the user, send them to
/// [`verification_url`](Self::verification_url), and poll with
/// [`device_code`](Self::device_code).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceCode {
    /// code used when polling for the token.
    pub device_code: Option<String>,
    /// code the user enters at the verification url.
    pub user_code: Option<String>,
    /// url the user visits to confirm the login.
    pub verification_url: Option<String>,
    /// lifetime of the codes, in seconds.
    pub expires_in: Option<u64>,
    /// recommended polling interval, in seconds.
    pub interval: Option<u64>,
}

/// an oauth token issued once the user confirms the device login.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OAuthToken {
    /// the access token, used as the client token.
    pub access_token: Option<String>,
    /// refresh token, when provided.
    pub refresh_token: Option<String>,
    /// token lifetime, in seconds.
    pub expires_in: Option<u64>,
    /// token type, typically `bearer`.
    pub token_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_codes_arrive_in_snake_case_unlike_the_rest_of_the_api() {
        let code: DeviceCode = serde_json::from_str(
            r#"{"device_code":"d","user_code":"ABCD-1234","verification_url":"https://ya.ru/device","expires_in":300,"interval":5}"#,
        )
        .unwrap();
        assert_eq!(code.device_code.as_deref(), Some("d"));
        assert_eq!(code.user_code.as_deref(), Some("ABCD-1234"));
        assert_eq!(code.expires_in, Some(300));
        assert_eq!(code.interval, Some(5));
    }

    #[test]
    fn tokens_arrive_in_snake_case_too() {
        let token: OAuthToken = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","expires_in":31536000,"token_type":"bearer"}"#,
        )
        .unwrap();
        assert_eq!(token.access_token.as_deref(), Some("a"));
        assert_eq!(token.refresh_token.as_deref(), Some("r"));
        assert_eq!(token.token_type.as_deref(), Some("bearer"));
    }

    #[test]
    fn device_auth_models_survive_an_empty_object() {
        serde_json::from_str::<DeviceCode>("{}").unwrap();
        serde_json::from_str::<OAuthToken>("{}").unwrap();
    }
}
