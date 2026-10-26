//! oauth device flow.
//!
//! lets an application obtain a token without embedding a browser: request a
//! code, show it to the user, poll until they confirm. the oauth host is not the
//! api origin, so these calls bypass the client's `base_url`.

use std::time::Duration;

use rand::Rng;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{DeviceCode, OAuthToken};
use crate::params::Params;

/// public oauth credentials of the official yandex music android app.
///
/// these are not secrets — they ship inside a publicly downloadable apk — but
/// requests made with them present as that app. override them via
/// [`DeviceAuthOptions`] if you have your own registered client.
pub const DEFAULT_CLIENT_ID: &str = "23cabbbdc6cd418abb4b39c32c41195d";
/// see [`DEFAULT_CLIENT_ID`].
pub const DEFAULT_CLIENT_SECRET: &str = "53bc75238f0c4d08a118e51fe9203300";

const OAUTH_BASE_URL: &str = "https://oauth.yandex.ru";
const DEFAULT_DEVICE_NAME: &str = "yamuse";

/// options for the device flow.
#[derive(Debug, Clone)]
pub struct DeviceAuthOptions {
    /// device id. defaults to a random 10-character string.
    pub device_id: Option<String>,
    /// human-readable device name.
    pub device_name: String,
    /// oauth client id.
    pub client_id: String,
    /// oauth client secret.
    pub client_secret: String,
    /// polling interval. defaults to the server's hint, floored at 3s.
    pub poll_interval: Option<Duration>,
    /// overall timeout. defaults to the code's own lifetime.
    pub timeout: Option<Duration>,
}

impl Default for DeviceAuthOptions {
    fn default() -> Self {
        Self {
            device_id: None,
            device_name: DEFAULT_DEVICE_NAME.to_owned(),
            client_id: DEFAULT_CLIENT_ID.to_owned(),
            client_secret: DEFAULT_CLIENT_SECRET.to_owned(),
            poll_interval: None,
            timeout: None,
        }
    }
}

/// generate a random 10-character device id.
#[must_use]
pub fn random_device_id() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..10)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

impl Client {
    /// request a device and user code pair, starting the device flow.
    ///
    /// `POST https://oauth.yandex.ru/device/code`
    ///
    /// # errors
    ///
    /// returns [`Error::DeviceAuth`] when the response carries no device code.
    pub async fn request_device_code(&self, options: &DeviceAuthOptions) -> Result<DeviceCode> {
        let device_id = options.device_id.clone().unwrap_or_else(random_device_id);
        let form = Params::new()
            .add("client_id", &options.client_id)
            .add("device_id", device_id)
            .add("device_name", &options.device_name);

        let value = self
            .post_form_absolute(&format!("{OAUTH_BASE_URL}/device/code"), form)
            .await?;
        let code: DeviceCode = serde_json::from_value(value)
            .map_err(|e| Error::DeviceAuth(format!("could not parse the device code: {e}")))?;
        if code.device_code.is_none() {
            return Err(Error::DeviceAuth(
                "the response carried no device code".into(),
            ));
        }
        Ok(code)
    }

    /// poll once for the token of a pending device authorization.
    ///
    /// `POST https://oauth.yandex.ru/token`
    ///
    /// returns `Ok(None)` while the user has not confirmed yet — that is the
    /// expected state, not an error, so a polling loop can just keep going.
    ///
    /// # errors
    ///
    /// returns [`Error::DeviceAuth`] on any other oauth rejection, such as an
    /// expired or denied code.
    pub async fn poll_device_token(
        &self,
        device_code: &str,
        options: &DeviceAuthOptions,
    ) -> Result<Option<OAuthToken>> {
        let form = Params::new()
            .add("grant_type", "device_code")
            .add("code", device_code)
            .add("client_id", &options.client_id)
            .add("client_secret", &options.client_secret);

        match self
            .post_form_absolute(&format!("{OAUTH_BASE_URL}/token"), form)
            .await
        {
            Ok(value) => Ok(Some(serde_json::from_value(value).map_err(|e| {
                Error::DeviceAuth(format!("could not parse the token: {e}"))
            })?)),
            Err(Error::BadRequest(message)) if message.contains("authorization_pending") => {
                Ok(None)
            },
            Err(Error::BadRequest(message)) => Err(Error::DeviceAuth(message)),
            Err(other) => Err(other),
        }
    }

    /// exchange a refresh token for a fresh access token.
    ///
    /// `POST https://oauth.yandex.ru/token`
    ///
    /// device-flow tokens expire. the response usually rotates the refresh token
    /// too, so persist the new one from the result rather than reusing the old.
    ///
    /// # errors
    ///
    /// returns [`Error::DeviceAuth`] when the grant is rejected or the response
    /// carries no access token.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        options: &DeviceAuthOptions,
    ) -> Result<OAuthToken> {
        let form = Params::new()
            .add("grant_type", "refresh_token")
            .add("refresh_token", refresh_token)
            .add("client_id", &options.client_id)
            .add("client_secret", &options.client_secret);

        let value = match self
            .post_form_absolute(&format!("{OAUTH_BASE_URL}/token"), form)
            .await
        {
            Ok(value) => value,
            Err(Error::BadRequest(message)) => return Err(Error::DeviceAuth(message)),
            Err(other) => return Err(other),
        };

        let token: OAuthToken = serde_json::from_value(value)
            .map_err(|e| Error::DeviceAuth(format!("could not parse the token: {e}")))?;
        if token.access_token.is_none() {
            return Err(Error::DeviceAuth(
                "the response carried no access token".into(),
            ));
        }
        Ok(token)
    }

    /// run the whole device flow: request a code, hand it to `on_code`, then poll
    /// until the user confirms.
    ///
    /// # errors
    ///
    /// returns [`Error::DeviceAuth`] on timeout or an oauth rejection.
    ///
    /// # examples
    ///
    /// ```no_run
    /// # async fn run() -> Result<(), yamuse::Error> {
    /// let client = yamuse::Client::anonymous()?;
    /// let token = client
    ///     .device_auth(Default::default(), |code| {
    ///         println!("enter {} at {}",
    ///             code.user_code.as_deref().unwrap_or(""),
    ///             code.verification_url.as_deref().unwrap_or(""));
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn device_auth(
        &self,
        options: DeviceAuthOptions,
        on_code: impl FnOnce(&DeviceCode),
    ) -> Result<OAuthToken> {
        let code = self.request_device_code(&options).await?;
        on_code(&code);

        let device_code = code
            .device_code
            .as_deref()
            .ok_or_else(|| Error::DeviceAuth("the response carried no device code".into()))?;

        // never poll faster than every 3s: the oauth host rate-limits, and a tight
        // loop turns a pending login into a rejected one.
        let interval = options
            .poll_interval
            .unwrap_or_else(|| Duration::from_secs(code.interval.unwrap_or(5)))
            .max(Duration::from_secs(3));
        let timeout = options
            .timeout
            .unwrap_or_else(|| Duration::from_secs(code.expires_in.unwrap_or(300)));
        let deadline = std::time::Instant::now() + timeout;

        loop {
            if std::time::Instant::now() >= deadline {
                return Err(Error::DeviceAuth("device authorization timed out".into()));
            }
            if let Some(token) = self.poll_device_token(device_code, &options).await? {
                return Ok(token);
            }
            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_ids_are_ten_alphanumeric_characters() {
        let id = random_device_id();
        assert_eq!(id.len(), 10);
        assert!(id.chars().all(char::is_alphanumeric));
    }

    #[test]
    fn device_ids_differ_between_calls() {
        // a fixed id would make two devices collide in the oauth session
        let ids: std::collections::HashSet<String> = (0..20).map(|_| random_device_id()).collect();
        assert!(
            ids.len() > 15,
            "expected mostly-unique ids, got {}",
            ids.len()
        );
    }

    #[test]
    fn the_defaults_use_the_android_client_credentials() {
        let options = DeviceAuthOptions::default();
        assert_eq!(options.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(options.client_secret, DEFAULT_CLIENT_SECRET);
        assert_eq!(options.device_name, "yamuse");
        assert!(options.device_id.is_none());
        assert!(options.poll_interval.is_none());
        assert!(options.timeout.is_none());
    }
}
