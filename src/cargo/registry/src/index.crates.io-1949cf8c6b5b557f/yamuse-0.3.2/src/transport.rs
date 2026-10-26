//! http transport.
//!
//! wraps `reqwest` with the three things every yandex music call needs and
//! nothing else: the android-client headers, unwrapping of the
//! `{ "result": ... }` envelope, and mapping failures onto [`Error`].
//!
//! retries are deliberately limited to idempotent `GET`s. a retried `POST` to
//! `likes/tracks/add` is harmless, but a retried `playlists/create` is not, so
//! mutations get exactly one attempt.

use std::time::Duration;

use rand::Rng;
#[cfg(feature = "download")]
use reqwest::Response;
use reqwest::header::{
    ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, RETRY_AFTER,
    USER_AGENT,
};
use reqwest::{Client as HttpClient, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::params::Params;
use crate::tolerant::FieldRepair;

/// `user-agent` sent with every request.
pub const DEFAULT_USER_AGENT: &str = concat!("yamuse/", env!("CARGO_PKG_VERSION"));

/// the android client version the api expects to see.
pub const CLIENT_HEADER: &str = "YandexMusicAndroid/24023621";

/// default per-request timeout for json calls.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// default timeout for raw downloads — audio, especially lossless, far outlasts
/// the json budget.
pub const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

const HEADER_MUSIC_CLIENT: &str = "x-yandex-music-client";

/// how a request body should be encoded.
#[derive(Debug, Clone)]
pub(crate) enum Body {
    /// `application/x-www-form-urlencoded`, built from [`Params`].
    Form(Params),
    /// `application/json`, pre-serialized.
    Json(String),
    /// no body at all.
    Empty,
}

/// retry policy for idempotent requests.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// how many times to retry after the first attempt. `0` disables retries.
    pub retries: u32,
    /// initial backoff.
    pub base: Duration,
    /// backoff ceiling.
    pub max: Duration,
    /// cap on a delay the server asked for in a `retry-after` header.
    ///
    /// a rate-limited response can name any delay it likes, and blocking a call
    /// for the hour one of them asks for is worse than trying again sooner and
    /// being told no twice. the header is honoured up to this ceiling.
    pub retry_after_max: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            retries: 2,
            base: Duration::from_millis(300),
            max: Duration::from_millis(4000),
            retry_after_max: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// exponential backoff with full jitter: `random(0..min(max, base * 2^attempt))`.
    ///
    /// full jitter rather than a fixed ramp so a fleet of clients recovering from
    /// the same outage spreads out instead of retrying in lockstep.
    fn backoff(&self, attempt: u32) -> Duration {
        let exp = self.base.saturating_mul(1u32 << attempt.min(16));
        let ceiling = exp.min(self.max);
        let millis = u64::try_from(ceiling.as_millis()).unwrap_or(u64::MAX);
        Duration::from_millis(rand::rng().random_range(0..=millis))
    }

    /// how long to wait before `attempt`, preferring what the server asked for.
    ///
    /// a server-named delay is used as-is up to [`retry_after_max`](Self::retry_after_max):
    /// jittering it would be second-guessing an explicit instruction, and waiting
    /// less than asked is what got us rate limited to begin with.
    fn delay_before(&self, attempt: u32, requested: Option<Duration>) -> Duration {
        requested.map_or_else(
            || self.backoff(attempt),
            |requested| requested.min(self.retry_after_max),
        )
    }
}

/// the configured http layer shared by every api call.
#[derive(Clone)]
pub struct Transport {
    http: HttpClient,
    headers: HeaderMap,
    timeout: Duration,
    download_timeout: Duration,
    retry: RetryPolicy,
    on_field_repair: Option<FieldRepairHook>,
}

// hand-written because the hook is a closure, and because printing the headers
// would put the oauth token in whatever a caller is debugging.
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("timeout", &self.timeout)
            .field("download_timeout", &self.download_timeout)
            .field("retry", &self.retry)
            .field("on_field_repair", &self.on_field_repair.is_some())
            .finish_non_exhaustive()
    }
}

/// notified whenever a response field had to be repaired to fit its model.
///
/// wire it to your logger to hear about upstream drift as it happens; see
/// [`crate::tolerant`].
pub type FieldRepairHook = std::sync::Arc<dyn Fn(&FieldRepair) + Send + Sync>;

impl Transport {
    /// build a transport from an already-configured `reqwest` client.
    ///
    /// # errors
    ///
    /// returns [`Error::InvalidResponse`] when the token or language contains
    /// bytes that are not valid in an http header.
    pub(crate) fn new(
        http: HttpClient,
        token: Option<&str>,
        language: &str,
        user_agent: &str,
        extra: &[(String, String)],
        timeout: Duration,
        download_timeout: Duration,
        retry: RetryPolicy,
        on_field_repair: Option<FieldRepairHook>,
    ) -> Result<Self> {
        // one helper per call site would need one closure per error type; a generic
        // fn covers every `InvalidHeader*` at once.
        fn invalid<E>(what: &'static str) -> impl FnOnce(E) -> Error {
            move |_| Error::InvalidResponse(format!("invalid {what}"))
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(HEADER_MUSIC_CLIENT),
            HeaderValue::from_static(CLIENT_HEADER),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent).map_err(invalid("user agent"))?,
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_str(language).map_err(invalid("language"))?,
        );
        if let Some(token) = token {
            let mut value =
                HeaderValue::from_str(&format!("OAuth {token}")).map_err(invalid("token"))?;
            value.set_sensitive(true);
            headers.insert(AUTHORIZATION, value);
        }
        for (name, value) in extra {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).map_err(invalid("header name"))?,
                HeaderValue::from_str(value).map_err(invalid("header value"))?,
            );
        }

        Ok(Self {
            http,
            headers,
            timeout,
            download_timeout,
            retry,
            on_field_repair,
        })
    }

    /// the underlying `reqwest` client, for callers that need to reuse the pool.
    #[must_use]
    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    /// `GET` a json endpoint and deserialize the unwrapped `result`.
    pub(crate) async fn get<T: DeserializeOwned>(&self, url: &str, params: &Params) -> Result<T> {
        let value = self.get_value(url, params, &[]).await?;
        self.decode(value)
    }

    /// deserialize a payload, tolerating a field whose type changed upstream and
    /// reporting each one to the hook when configured.
    pub(crate) fn decode<T: DeserializeOwned>(&self, value: Value) -> Result<T> {
        let (model, dropped) = crate::tolerant::deserialize(value)?;
        if let Some(hook) = &self.on_field_repair {
            for field in &dropped {
                hook(field);
            }
        }
        Ok(model)
    }

    /// `GET` a json endpoint with extra per-call headers.
    pub(crate) async fn get_with_headers<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &Params,
        headers: &[(&str, &str)],
    ) -> Result<T> {
        let value = self.get_value(url, params, headers).await?;
        self.decode(value)
    }

    /// `GET` a json endpoint and return the unwrapped `result` as raw json.
    pub(crate) async fn get_value(
        &self,
        url: &str,
        params: &Params,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        // GET is idempotent, so transient failures are safe to retry.
        let mut attempt = 0;
        loop {
            let result = self
                .send(
                    Method::GET,
                    url,
                    params,
                    &Body::Empty,
                    headers,
                    self.timeout,
                )
                .await;
            match result {
                Ok(bytes) => return unwrap_envelope(&bytes),
                Err(err) if attempt < self.retry.retries && err.is_retryable() => {
                    let delay = self.retry.delay_before(attempt, err.retry_after());
                    crate::log::warning!(
                        url = crate::log::redacted(url),
                        attempt = attempt + 1,
                        of = self.retry.retries,
                        delay_ms = delay.as_millis(),
                        server_asked = err.retry_after().is_some(),
                        "retrying: {err}",
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                },
                Err(err) => return Err(err),
            }
        }
    }

    /// `POST` a form-encoded body and deserialize the unwrapped `result`.
    pub(crate) async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        form: Params,
    ) -> Result<T> {
        let value = self.post_form_value(url, &Params::new(), form, &[]).await?;
        self.decode(value)
    }

    /// `POST` a form-encoded body and return the unwrapped `result` as raw json.
    ///
    /// several endpoints want some of their parameters in the query and the rest
    /// in the body, so both are taken here rather than being spliced into the url
    /// by the caller.
    pub(crate) async fn post_form_value(
        &self,
        url: &str,
        query: &Params,
        form: Params,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        let bytes = self
            .send(
                Method::POST,
                url,
                query,
                &Body::Form(form),
                headers,
                self.timeout,
            )
            .await?;
        unwrap_envelope(&bytes)
    }

    /// `POST` a json body and return the unwrapped `result` as raw json.
    pub(crate) async fn post_json_value(
        &self,
        url: &str,
        json: &Value,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        let body = Body::Json(serde_json::to_string(json)?);
        let bytes = self
            .send(
                Method::POST,
                url,
                &Params::new(),
                &body,
                headers,
                self.timeout,
            )
            .await?;
        unwrap_envelope(&bytes)
    }

    /// `POST` a pre-serialized json body (queue payloads arrive as strings).
    pub(crate) async fn post_raw_json_value(
        &self,
        url: &str,
        json: String,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        let bytes = self
            .send(
                Method::POST,
                url,
                &Params::new(),
                &Body::Json(json),
                headers,
                self.timeout,
            )
            .await?;
        unwrap_envelope(&bytes)
    }

    /// `PUT` a json body and return the unwrapped `result` as raw json.
    pub(crate) async fn put_value(&self, url: &str, json: &Value) -> Result<Value> {
        let body = Body::Json(serde_json::to_string(json)?);
        let bytes = self
            .send(Method::PUT, url, &Params::new(), &body, &[], self.timeout)
            .await?;
        unwrap_envelope(&bytes)
    }

    /// `DELETE` with a json body and return the unwrapped `result` as raw json.
    pub(crate) async fn delete_value(&self, url: &str, json: &Value) -> Result<Value> {
        let body = Body::Json(serde_json::to_string(json)?);
        let bytes = self
            .send(
                Method::DELETE,
                url,
                &Params::new(),
                &body,
                &[],
                self.timeout,
            )
            .await?;
        unwrap_envelope(&bytes)
    }

    /// `GET` raw bytes without json parsing — download-info xml, lyrics text, audio.
    pub async fn retrieve(&self, url: &str) -> Result<Vec<u8>> {
        self.send(
            Method::GET,
            url,
            &Params::new(),
            &Body::Empty,
            &[],
            self.download_timeout,
        )
        .await
    }

    /// `GET` raw bytes and decode them as utf-8, lossily.
    pub(crate) async fn retrieve_text(&self, url: &str) -> Result<String> {
        let bytes = self.retrieve(url).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// open a `GET` for streaming, validating the status before returning.
    #[cfg(feature = "download")]
    pub(crate) async fn open_stream(&self, url: &str) -> Result<Response> {
        let response = self
            .http
            .get(url)
            .headers(self.headers.clone())
            .timeout(self.download_timeout)
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let requested = retry_after(response.headers());
        let bytes = response.bytes().await.unwrap_or_default();
        Err(Error::from_status(
            status.as_u16(),
            extract_error_message(&bytes, status),
            requested,
        ))
    }

    /// perform one request and return the raw response body.
    async fn send(
        &self,
        method: Method,
        url: &str,
        query: &Params,
        body: &Body,
        headers: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        // only the log line needs this, and `method` is moved into the builder below
        #[cfg(feature = "tracing")]
        let method_name = method.as_str().to_owned();
        let mut request = self
            .http
            .request(method, url)
            .headers(self.headers.clone())
            .timeout(timeout);

        if !query.is_empty() {
            request = request.query(&query.pairs().collect::<Vec<_>>());
        }
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request = match body {
            Body::Form(form) => request
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .form(&form.pairs().collect::<Vec<_>>()),
            Body::Json(json) => request
                .header(CONTENT_TYPE, "application/json")
                .body(json.clone()),
            Body::Empty => request,
        };

        let response = request.send().await?;
        let status = response.status();
        // read `retry-after` before the body: consuming the response moves it
        let requested = retry_after(response.headers());
        // read the body under the same deadline: a stalled body stream with no tcp
        // reset would otherwise hang past the timeout.
        let bytes = response.bytes().await?;

        crate::log::debug!(
            method = method_name,
            url = crate::log::redacted(url),
            status = status.as_u16(),
            bytes = bytes.len(),
            "request",
        );

        if status.is_success() {
            return Ok(bytes.to_vec());
        }
        Err(Error::from_status(
            status.as_u16(),
            extract_error_message(&bytes, status),
            requested,
        ))
    }
}

/// the delay a response's `retry-after` header asked for.
///
/// only the delta-seconds form is understood. the header also allows an http-date,
/// which would need a date parser this crate deliberately does not carry — and
/// this api sends seconds.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let seconds: u64 = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds))
}

/// unwrap the `{ "result": ... }` envelope the api wraps successful payloads in.
///
/// detection is by key presence, not truthiness, so a legitimate `result: null`
/// survives. responses without the envelope (oauth endpoints) pass through.
fn unwrap_envelope(bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|e| Error::InvalidResponse(format!("response is not json: {e}")))?;
    match value {
        Value::Object(mut map) => Ok(map.remove("result").unwrap_or(Value::Object(map))),
        other => Ok(other),
    }
}

/// pull a human-readable message out of an error body.
///
/// the api reports failures in three shapes, in descending specificity: a
/// `{name, message}` nested under `result` (newer services such as `/queues`), the
/// same pair at the top level, and the legacy oauth `{error, error_description}`.
fn extract_error_message(bytes: &[u8], status: StatusCode) -> String {
    let fallback = || format!("http {}", status.as_u16());
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return fallback();
    };
    let Some(object) = value.as_object() else {
        return fallback();
    };

    let named = |v: Option<&Value>| -> Option<String> {
        let map = v?.as_object()?;
        let name = map.get("name").and_then(Value::as_str).unwrap_or_default();
        let message = map
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let joined = [name, message]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>();
        (!joined.is_empty()).then(|| joined.join(": "))
    };

    if let Some(found) = named(object.get("result"))
        .or_else(|| named(Some(&value)))
        .or_else(|| named(object.get("error")))
    {
        return found;
    }

    // legacy `{ error, errorDescription }` / oauth snake_case `error_description`
    let error = object
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let description = object
        .get("errorDescription")
        .or_else(|| object.get("error_description"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let joined = [error, description]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");

    if joined.is_empty() {
        fallback()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(body: &str) -> String {
        extract_error_message(body.as_bytes(), StatusCode::BAD_REQUEST)
    }

    #[test]
    fn envelope_is_unwrapped() {
        let value = unwrap_envelope(br#"{"result":{"id":42},"invocationInfo":{}}"#).unwrap();
        assert_eq!(value, serde_json::json!({"id": 42}));
    }

    #[test]
    fn a_null_result_is_preserved_not_treated_as_missing() {
        assert_eq!(unwrap_envelope(br#"{"result":null}"#).unwrap(), Value::Null);
    }

    #[test]
    fn unenveloped_objects_pass_through() {
        // oauth endpoints answer without the envelope
        let value = unwrap_envelope(br#"{"access_token":"abc","expires_in":3600}"#).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"access_token": "abc", "expires_in": 3600})
        );
    }

    #[test]
    fn bare_values_pass_through() {
        assert_eq!(
            unwrap_envelope(b"\"ok\"").unwrap(),
            Value::String("ok".into())
        );
        assert_eq!(
            unwrap_envelope(b"[1,2]").unwrap(),
            serde_json::json!([1, 2])
        );
    }

    #[test]
    fn an_empty_body_becomes_null() {
        assert_eq!(unwrap_envelope(b"").unwrap(), Value::Null);
    }

    #[test]
    fn non_json_bodies_are_rejected() {
        assert!(matches!(
            unwrap_envelope(b"<html>"),
            Err(Error::InvalidResponse(_))
        ));
    }

    #[test]
    fn nested_result_errors_win_over_top_level() {
        let body =
            r#"{"result":{"name":"Inner","message":"nested"},"name":"Outer","message":"top"}"#;
        assert_eq!(message(body), "Inner: nested");
    }

    #[test]
    fn top_level_name_message_is_used_when_there_is_no_nested_one() {
        assert_eq!(
            message(r#"{"name":"Bad","message":"broken"}"#),
            "Bad: broken"
        );
    }

    #[test]
    fn a_lone_message_needs_no_name() {
        assert_eq!(message(r#"{"message":"just a message"}"#), "just a message");
    }

    #[test]
    fn legacy_oauth_shapes_are_understood() {
        assert_eq!(
            message(r#"{"error":"authorization_pending","error_description":"waiting"}"#),
            "authorization_pending waiting"
        );
        assert_eq!(
            message(r#"{"error":"invalid_grant","errorDescription":"expired"}"#),
            "invalid_grant expired"
        );
        assert_eq!(message(r#"{"error":"solo_error"}"#), "solo_error");
    }

    #[test]
    fn unrecognized_bodies_fall_back_to_the_status() {
        assert_eq!(message("{}"), "http 400");
        assert_eq!(message("not json at all"), "http 400");
        assert_eq!(message("[]"), "http 400");
        // an `error` object without name/message still falls back
        assert_eq!(message(r#"{"error":{"code":7}}"#), "http 400");
    }

    #[test]
    fn backoff_is_bounded_by_the_ceiling() {
        let policy = RetryPolicy {
            retries: 5,
            base: Duration::from_millis(100),
            max: Duration::from_millis(1000),
            ..RetryPolicy::default()
        };
        for attempt in 0..8 {
            assert!(policy.backoff(attempt) <= policy.max);
        }
    }

    #[test]
    fn backoff_never_overflows_on_absurd_attempts() {
        let policy = RetryPolicy {
            retries: 1,
            base: Duration::from_secs(1),
            max: Duration::from_secs(30),
            ..RetryPolicy::default()
        };
        assert!(policy.backoff(u32::MAX) <= policy.max);
    }

    #[test]
    fn a_server_named_delay_wins_over_the_backoff_ramp() {
        let policy = RetryPolicy {
            retries: 3,
            base: Duration::from_millis(10),
            max: Duration::from_millis(20),
            retry_after_max: Duration::from_secs(30),
        };
        // used verbatim: jittering an explicit instruction would be second-guessing it
        assert_eq!(
            policy.delay_before(0, Some(Duration::from_secs(5))),
            Duration::from_secs(5)
        );
        // and capped, because this api does ask for delays of an hour
        assert_eq!(
            policy.delay_before(0, Some(Duration::from_secs(3600))),
            Duration::from_secs(30)
        );
        // without one, the jittered ramp still applies
        assert!(policy.delay_before(0, None) <= policy.max);
    }

    fn headers_with(name: HeaderName, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn retry_after_is_read_as_delta_seconds() {
        assert_eq!(
            retry_after(&headers_with(RETRY_AFTER, "12")),
            Some(Duration::from_secs(12))
        );
        assert_eq!(
            retry_after(&headers_with(RETRY_AFTER, " 3 ")),
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            retry_after(&headers_with(RETRY_AFTER, "0")),
            Some(Duration::ZERO)
        );
        assert_eq!(retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn an_http_date_retry_after_is_ignored_rather_than_guessed_at() {
        // the header allows a date; parsing one would need a date parser this
        // crate does not carry, so the jittered ramp takes over instead.
        assert_eq!(
            retry_after(&headers_with(RETRY_AFTER, "Wed, 21 Oct 2026 07:28:00 GMT")),
            None
        );
        assert_eq!(retry_after(&headers_with(RETRY_AFTER, "-1")), None);
    }
}
