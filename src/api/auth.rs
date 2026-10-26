//! Session bootstrap: validate a token, refresh it when possible, and build a
//! client with a populated account.
//!
//! All failures are classified so the app can distinguish "ask the listener to
//! sign in again" from "the network is down; retry with backoff" without parsing
//! human-readable errors.

use std::time::{SystemTime, UNIX_EPOCH};

use yamuse::models::Status;
use yamuse::{Client, DeviceAuthOptions, Error as YamuseError, Language};

use crate::model::Profile;

/// A validated API client and the credentials that go with it.
pub struct SessionBootstrap {
    pub client: Client,
    pub profile: Profile,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    /// True when a refresh token was exchanged during this bootstrap.
    pub refreshed: bool,
}

/// A classified bootstrap failure.
#[derive(Debug, Clone)]
pub struct BootstrapFailure {
    pub message: String,
    /// The listener must supply a new token.
    pub needs_auth: bool,
    /// Trying again later could succeed without user action.
    pub retryable: bool,
}

impl BootstrapFailure {
    fn auth(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            needs_auth: true,
            retryable: false,
        }
    }

    fn transient(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            needs_auth: false,
            retryable: true,
        }
    }

    fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            needs_auth: false,
            retryable: false,
        }
    }
}

/// Connect using stored credentials.
///
/// Tokens close to expiry are refreshed eagerly; otherwise a 401 triggers one
/// refresh attempt. The resulting token rotation is returned to the caller so it
/// can be persisted.
pub async fn connect(
    access_token: String,
    refresh_token: String,
    expires_at: i64,
) -> Result<SessionBootstrap, BootstrapFailure> {
    let now = unix_now();
    let expires_soon = expires_at > 0 && expires_at <= now + 60;
    let mut refreshed = false;
    let mut access_token = access_token.trim().to_owned();
    let mut refresh_token = refresh_token.trim().to_owned();
    let mut expires_at = expires_at;

    if (access_token.is_empty() || expires_soon) && !refresh_token.is_empty() {
        match refresh_access_token(&refresh_token).await {
            Ok(token) => {
                access_token = token.access_token;
                refresh_token = token.refresh_token.unwrap_or(refresh_token);
                expires_at = token.expires_at;
                refreshed = true;
            }
            Err(err) => {
                if access_token.is_empty() {
                    return Err(err);
                }
                // The access token may still be usable; fall through and let the
                // catalogue request decide.
            }
        }
    }

    if access_token.is_empty() {
        return Err(BootstrapFailure::auth(
            "No Yandex Music token is configured. Sign in to continue.",
        ));
    }

    match initialise(&access_token).await {
        Ok((client, status)) => Ok(SessionBootstrap {
            profile: profile_from_status(&status),
            client,
            access_token,
            refresh_token,
            expires_at,
            refreshed,
        }),
        Err(err) if is_unauthorized(&err) && !refresh_token.is_empty() && !refreshed => {
            let token = refresh_access_token(&refresh_token).await?;
            access_token = token.access_token;
            refresh_token = token.refresh_token.unwrap_or(refresh_token);
            expires_at = token.expires_at;
            let (client, status) = initialise(&access_token).await.map_err(classify_error)?;
            Ok(SessionBootstrap {
                profile: profile_from_status(&status),
                client,
                access_token,
                refresh_token,
                expires_at,
                refreshed: true,
            })
        }
        Err(err) if is_unauthorized(&err) => Err(BootstrapFailure::auth(
            "The token was rejected. Sign in again to continue.",
        )),
        Err(err) => Err(classify_error(err)),
    }
}

pub(crate) async fn initialise(token: &str) -> Result<(Client, Status), YamuseError> {
    let client = Client::builder()
        .token(token)
        .language(Language::Ru)
        .user_agent(concat!("yandex-music-tui/", env!("CARGO_PKG_VERSION")))
        .build()?;
    client.init().await?;
    let status = client.me().unwrap_or_default();
    Ok((client, status))
}

pub(crate) struct RefreshedToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64,
}

pub(crate) async fn refresh_access_token(
    refresh_token: &str,
) -> Result<RefreshedToken, BootstrapFailure> {
    let anonymous = Client::anonymous().map_err(classify_error)?;
    let options = DeviceAuthOptions {
        device_name: "yandex-music-tui".to_owned(),
        ..DeviceAuthOptions::default()
    };
    let token = anonymous
        .refresh_token(refresh_token, &options)
        .await
        .map_err(|err| match err {
            YamuseError::DeviceAuth(message) => {
                BootstrapFailure::auth(format!("Could not refresh the session: {message}"))
            }
            other => classify_error(other),
        })?;
    let access_token = token.access_token.ok_or_else(|| {
        BootstrapFailure::auth("The refresh response did not include an access token.")
    })?;
    let expires_at = token
        .expires_in
        .map(|seconds| unix_now() + seconds as i64)
        .unwrap_or(0);
    Ok(RefreshedToken {
        access_token,
        refresh_token: token.refresh_token,
        expires_at,
    })
}

fn classify_error(err: YamuseError) -> BootstrapFailure {
    match &err {
        YamuseError::Unauthorized(_) => {
            BootstrapFailure::auth("The token was rejected. Sign in again to continue.")
        }
        YamuseError::RateLimited { retry_after, .. } => BootstrapFailure::transient(format!(
            "Yandex Music is rate limiting requests; retrying{}. ",
            retry_after
                .map(|delay| format!(" after {}s", delay.as_secs()))
                .unwrap_or_default()
        )),
        other if other.is_retryable() => {
            BootstrapFailure::transient(format!("Could not reach Yandex Music: {other}"))
        }
        other => BootstrapFailure::permanent(format!(
            "Yandex Music returned an unexpected error: {other}"
        )),
    }
}

fn is_unauthorized(err: &YamuseError) -> bool {
    matches!(err, YamuseError::Unauthorized(_))
}

fn profile_from_status(status: &Status) -> Profile {
    let account = status.account.as_ref();
    let login = account
        .and_then(|account| account.login.clone())
        .or_else(|| status.default_email.clone())
        .unwrap_or_default();
    let display_name = account
        .and_then(|account| {
            account
                .display_name
                .clone()
                .or_else(|| account.full_name.clone())
                .or_else(|| account.first_name.clone())
        })
        .unwrap_or_default();
    Profile {
        login,
        display_name,
        has_plus: status.has_plus(),
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_classified_by_user_action() {
        let auth = classify_error(YamuseError::Unauthorized("nope".into()));
        assert!(auth.needs_auth);
        assert!(!auth.retryable);

        let network = classify_error(YamuseError::Network("dns".into()));
        assert!(!network.needs_auth);
        assert!(network.retryable);

        let bad = classify_error(YamuseError::BadRequest("bad".into()));
        assert!(!bad.needs_auth);
        assert!(!bad.retryable);
    }
}
