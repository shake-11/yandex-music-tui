//! the client and its builder.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::models::{Mutation, Status};
use crate::params::Params;
use crate::transport::{
    DEFAULT_DOWNLOAD_TIMEOUT, DEFAULT_TIMEOUT, DEFAULT_USER_AGENT, RetryPolicy, Transport,
};
use crate::types::{BatchObject, Language};

/// default api origin.
pub const DEFAULT_BASE_URL: &str = "https://api.music.yandex.net";

/// default device descriptor sent with queue requests.
///
/// `device_id=random` and `uuid=random` are the literal strings the official
/// clients send, not placeholders this crate forgot to fill in — the api accepts
/// them and scopes the queue accordingly. pass
/// [`ClientBuilder::device`] to present a stable identity instead.
pub const DEFAULT_DEVICE: &str =
    "os=Rust; os_version=; manufacturer=yamuse; model=yamuse; clid=; device_id=random; uuid=random";

/// how many ids travel in one batch request.
///
/// the api has a size limit it does not document and answers with a 413 past it.
/// a library of several thousand liked tracks is ordinary, so long lists are
/// split rather than handed over whole and rejected.
pub const BATCH_CHUNK: usize = 250;

/// shared state behind a [`Client`]. cloning a client shares this.
#[derive(Debug)]
struct Inner {
    transport: Transport,
    base_url: String,
    language: Language,
    device: String,
    token: Option<String>,
    /// filled in by [`Client::init`]; some endpoints default their `userId` to it.
    account: RwLock<Option<Status>>,
}

/// a typed, async client for the yandex music api.
///
/// cloning is cheap — clones share one connection pool and one account cache — so
/// pass clones around rather than wrapping in another `Arc`.
///
/// # examples
///
/// ```no_run
/// use yamuse::Client;
///
/// # async fn run() -> Result<(), yamuse::Error> {
/// let client = Client::builder().token("your-oauth-token").build()?;
/// let tracks = client.tracks([2]).await?;
/// println!("{:?}", tracks.first().and_then(|t| t.title.as_deref()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    /// start configuring a client.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// build an anonymous client, without a token.
    ///
    /// enough for the device-auth endpoints and a few public ones; everything else
    /// answers 401.
    ///
    /// # errors
    ///
    /// returns [`Error::InvalidResponse`] when the default headers cannot be built.
    pub fn anonymous() -> Result<Self> {
        ClientBuilder::default().build()
    }

    /// load the account status, enabling endpoints that default to "me".
    ///
    /// call this once after building. without it, methods whose `user_id` is
    /// optional have no uid to fall back to and will fail.
    ///
    /// # errors
    ///
    /// returns a transport error, or [`Error::Unauthorized`] when the token is
    /// invalid.
    pub async fn init(&self) -> Result<&Self> {
        self.refresh_account().await?;
        Ok(self)
    }

    /// re-read the account status and replace what [`init`](Self::init) cached.
    ///
    /// the cache is populated once and never invalidates itself, because nothing
    /// in a response says it went stale. call this after changing a setting, or
    /// when a subscription may have been bought or expired elsewhere.
    ///
    /// # errors
    ///
    /// returns a transport error, or [`Error::Unauthorized`] when the token is
    /// invalid. the previously cached status is left in place on failure.
    pub async fn refresh_account(&self) -> Result<Status> {
        let status = self.account_status().await?;
        // the lock is only ever held for this write and for short uid reads
        if let Ok(mut guard) = self.inner.account.write() {
            *guard = Some(status.clone());
        }
        Ok(status)
    }

    /// the cached account status, once [`init`](Self::init) has run.
    #[must_use]
    pub fn me(&self) -> Option<Status> {
        self.inner.account.read().ok()?.clone()
    }

    /// the authenticated account's uid, once [`init`](Self::init) has run.
    #[must_use]
    pub fn account_uid(&self) -> Option<i64> {
        self.inner.account.read().ok()?.as_ref()?.uid()
    }

    /// the oauth token this client authenticates with, if any.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.inner.token.as_deref()
    }

    /// the api origin requests are built against.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.inner.base_url
    }

    /// the response language.
    #[must_use]
    pub fn language(&self) -> Language {
        self.inner.language
    }

    /// the device descriptor sent with queue requests.
    #[must_use]
    pub fn device(&self) -> &str {
        &self.inner.device
    }

    /// the http transport, for streaming downloads and raw retrievals.
    #[must_use]
    pub fn transport(&self) -> &Transport {
        &self.inner.transport
    }

    /// `GET` any endpoint and return the unwrapped `result` as raw json.
    ///
    /// the escape hatch for two things the typed methods cannot do: calling an
    /// endpoint this crate does not model yet, and seeing what the api actually
    /// sent. the latter is what [`drift::unmapped_fields`](crate::drift::unmapped_fields)
    /// needs — comparing a parsed model against the payload it came from is only
    /// possible if you can hold both.
    ///
    /// `path` is relative to the api origin and is *not* escaped; build it with
    /// values you trust, or prefer the typed method.
    ///
    /// # errors
    ///
    /// the same as any other `GET`: transport failures, and the api's own
    /// rejections mapped onto [`Error`].
    ///
    /// # examples
    ///
    /// ```no_run
    /// # async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
    /// use yamuse::{Params, drift::unmapped_fields, models::Track};
    ///
    /// let raw = client.get_raw("/tracks/42/supplement", Params::new()).await?;
    /// let parsed: yamuse::models::Supplement = serde_json::from_value(raw.clone())?;
    ///
    /// // anything the api sends that this crate does not model yet
    /// for field in unmapped_fields(&raw, &parsed)? {
    ///     println!("unmapped: {field}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_raw(&self, path: &str, params: Params) -> Result<Value> {
        self.get_value(path, params).await
    }

    // ── internals shared by the method modules ──────────────────────────────

    /// build an absolute url from a path relative to the api origin.
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.inner.base_url)
    }

    /// resolve the uid an endpoint should act on: the explicit one, else "me".
    ///
    /// # errors
    ///
    /// returns [`Error::IdMissing`] when no uid was given and [`init`](Self::init)
    /// has not run — a clearer failure than the 404 the api would answer.
    pub(crate) fn resolve_uid(&self, user_id: Option<i64>) -> Result<i64> {
        user_id
            .or_else(|| self.account_uid())
            .ok_or(Error::IdMissing(
                "user id: pass one explicitly or call init() first",
            ))
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str, params: Params) -> Result<T> {
        self.inner.transport.get(&self.url(path), &params).await
    }

    /// deserialize a payload the tolerant way, so a field whose type changed
    /// upstream costs that field rather than the whole call.
    pub(crate) fn decode<T: DeserializeOwned>(&self, value: Value) -> Result<T> {
        self.inner.transport.decode(value)
    }

    pub(crate) async fn get_value(&self, path: &str, params: Params) -> Result<Value> {
        self.inner
            .transport
            .get_value(&self.url(path), &params, &[])
            .await
    }

    pub(crate) async fn get_with_headers<T: DeserializeOwned>(
        &self,
        path: &str,
        params: Params,
        headers: &[(&str, &str)],
    ) -> Result<T> {
        self.inner
            .transport
            .get_with_headers(&self.url(path), &params, headers)
            .await
    }

    pub(crate) async fn post_form<T: DeserializeOwned>(
        &self,
        path: &str,
        form: Params,
    ) -> Result<T> {
        self.inner.transport.post_form(&self.url(path), form).await
    }

    pub(crate) async fn post_form_value(&self, path: &str, form: Params) -> Result<Value> {
        self.post_form_value_with(path, &Params::new(), form, &[])
            .await
    }

    /// `POST` a form body with query parameters and per-call headers.
    ///
    /// a handful of endpoints split their parameters across the query and the
    /// body; this is how they say so, rather than splicing a `?` into the path.
    pub(crate) async fn post_form_value_with(
        &self,
        path: &str,
        query: &Params,
        form: Params,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        self.inner
            .transport
            .post_form_value(&self.url(path), query, form, headers)
            .await
    }

    pub(crate) async fn post_json_value(&self, path: &str, json: &Value) -> Result<Value> {
        self.inner
            .transport
            .post_json_value(&self.url(path), json, &[])
            .await
    }

    pub(crate) async fn post_raw_json_value(
        &self,
        path: &str,
        json: String,
        headers: &[(&str, &str)],
    ) -> Result<Value> {
        self.inner
            .transport
            .post_raw_json_value(&self.url(path), json, headers)
            .await
    }

    pub(crate) async fn put_value(&self, path: &str, json: &Value) -> Result<Value> {
        self.inner.transport.put_value(&self.url(path), json).await
    }

    pub(crate) async fn delete_value(&self, path: &str, json: &Value) -> Result<Value> {
        self.inner
            .transport
            .delete_value(&self.url(path), json)
            .await
    }

    /// `POST` to an absolute url — the oauth host is not the api origin.
    pub(crate) async fn post_form_absolute(&self, url: &str, form: Params) -> Result<Value> {
        self.inner
            .transport
            .post_form_value(url, &Params::new(), form, &[])
            .await
    }

    /// fetch a batch of objects from a `/{type}s` endpoint.
    ///
    /// the api takes these ids as a repeated form field rather than in the path,
    /// which is why an "id lookup" is a `POST`. long id lists are split across
    /// several requests — see [`BATCH_CHUNK`].
    pub(crate) async fn get_batch<T: DeserializeOwned>(
        &self,
        object: BatchObject,
        ids: impl IntoIterator<Item = impl std::fmt::Display>,
        extra: Params,
    ) -> Result<Vec<T>> {
        let kind = object.as_str();
        let suffix = if object == BatchObject::Playlist {
            "/list"
        } else {
            ""
        };
        let path = format!("/{kind}s{suffix}");
        let key = format!("{kind}-ids");
        let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();

        // the common case is a handful of ids and exactly one request
        if ids.len() <= BATCH_CHUNK {
            return self.post_form(&path, extra.add_all(&key, ids)).await;
        }

        let mut all = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(BATCH_CHUNK) {
            let form = extra.clone().add_all(&key, chunk);
            all.extend(self.post_form::<Vec<T>>(&path, form).await?);
        }
        Ok(all)
    }

    /// apply a mutation to a long id list, one chunk at a time.
    ///
    /// unlike a batch read, this is not transparent: the chunks are separate
    /// requests, so a failure part-way leaves the earlier ones applied. the
    /// alternative is a request the api rejects outright, which leaves nothing
    /// applied but also does nothing useful.
    pub(crate) async fn mutate_batch(
        &self,
        path: &str,
        key: &str,
        ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Mutation> {
        let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();

        if ids.len() <= BATCH_CHUNK {
            let value = self
                .post_form_value(path, Params::new().add_all(key, ids))
                .await?;
            return Self::acknowledged(&value);
        }

        let mut last = Mutation::default();
        for chunk in ids.chunks(BATCH_CHUNK) {
            let value = self
                .post_form_value(path, Params::new().add_all(key, chunk))
                .await?;
            // the revision of the final chunk is the one that describes the
            // library once every chunk has landed
            last = Self::acknowledged(&value)?;
        }
        Ok(last)
    }

    /// read a mutation endpoint's acknowledgement, or fail.
    ///
    /// success has two shapes: most endpoints answer the bare string `"ok"`, while
    /// the track like/dislike endpoints answer an object carrying the new library
    /// `revision` instead.
    ///
    /// anything else is an error rather than a `false`. a boolean return could
    /// only say "that did not work" — without the reason, and quietly, since
    /// nobody checks a returned `bool`.
    pub(crate) fn acknowledged(value: &Value) -> Result<Mutation> {
        let rejected =
            || Error::InvalidResponse(format!("the api did not acknowledge the mutation: {value}"));

        match value {
            Value::String(marker) if marker == "ok" => Ok(Mutation::default()),
            Value::Object(map) => {
                if map.contains_key("revision") {
                    // present but unparseable still counts as acknowledged: the
                    // revision is a bonus, the acknowledgement is the point
                    return Ok(Mutation {
                        revision: map.get("revision").and_then(Value::as_i64),
                    });
                }
                if map.get("status") == Some(&Value::String("ok".into())) {
                    return Ok(Mutation::default());
                }
                Err(rejected())
            },
            _ => Err(rejected()),
        }
    }

    /// [`acknowledged`](Self::acknowledged) for the clip endpoints, whose success
    /// shape is not confirmed to be the one their siblings use.
    ///
    /// any object counts, rather than reporting a failure that may not be one.
    pub(crate) fn acknowledged_loosely(value: &Value) -> Result<Mutation> {
        match Self::acknowledged(value) {
            Ok(mutation) => Ok(mutation),
            Err(err) if value.is_object() => {
                let _ = err;
                Ok(Mutation::default())
            },
            Err(err) => Err(err),
        }
    }
}

/// builder for [`Client`].
#[derive(Clone)]
pub struct ClientBuilder {
    token: Option<String>,
    base_url: String,
    language: Language,
    device: String,
    user_agent: String,
    headers: Vec<(String, String)>,
    timeout: Duration,
    download_timeout: Duration,
    retry: RetryPolicy,
    http: Option<reqwest::Client>,
    on_field_repair: Option<crate::transport::FieldRepairHook>,
}

// hand-written because the dropped-field hook is a closure.
impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("base_url", &self.base_url)
            .field("language", &self.language)
            .field("device", &self.device)
            .field("user_agent", &self.user_agent)
            .field("timeout", &self.timeout)
            .field("download_timeout", &self.download_timeout)
            .field("retry", &self.retry)
            .field("has_token", &self.token.is_some())
            .field("on_field_repair", &self.on_field_repair.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            token: None,
            base_url: DEFAULT_BASE_URL.to_owned(),
            language: Language::default(),
            device: DEFAULT_DEVICE.to_owned(),
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            headers: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            download_timeout: DEFAULT_DOWNLOAD_TIMEOUT,
            retry: RetryPolicy::default(),
            http: None,
            on_field_repair: None,
        }
    }
}

impl ClientBuilder {
    /// authenticate with an oauth token.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// override the api origin — useful for pointing tests at a mock server.
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        base_url.truncate(base_url.trim_end_matches('/').len());
        self.base_url = base_url;
        self
    }

    /// set the response language.
    #[must_use]
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// set the device descriptor sent with queue requests.
    #[must_use]
    pub fn device(mut self, device: impl Into<String>) -> Self {
        self.device = device.into();
        self
    }

    /// override the `user-agent`.
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// add a header sent with every request.
    ///
    /// headers are applied after the ones this crate sets, so naming one of those
    /// replaces it — including `authorization`, `user-agent` and
    /// `x-yandex-music-client`. that is deliberate: presenting as a different
    /// client version is sometimes the only way past a rejection. it also means a
    /// name given twice keeps only the last value, rather than sending both.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// set the per-request timeout for json calls.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// set the timeout for raw downloads.
    #[must_use]
    pub fn download_timeout(mut self, timeout: Duration) -> Self {
        self.download_timeout = timeout;
        self
    }

    /// set the retry policy for idempotent requests.
    #[must_use]
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// supply a pre-configured `reqwest` client — proxies, custom tls, a shared
    /// connection pool.
    #[must_use]
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }

    /// be told when a response field had to be repaired because its type did not
    /// match the model.
    ///
    /// the api does change field types without notice, and this crate repairs just
    /// that field rather than failing the request (see [`crate::tolerant`]). that
    /// keeps things working, but silently — wire this to your logger so drift
    /// surfaces instead of accumulating.
    ///
    /// filter on [`FieldRepair::is_lossy`](crate::tolerant::FieldRepair::is_lossy)
    /// to ignore harmless number narrowing and hear only about real drift.
    ///
    /// # examples
    ///
    /// ```
    /// # fn main() -> Result<(), yamuse::Error> {
    /// let client = yamuse::Client::builder()
    ///     .token("t")
    ///     .on_field_repair(|repair| eprintln!("yandex music api: {repair}"))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn on_field_repair(
        mut self,
        hook: impl Fn(&crate::tolerant::FieldRepair) + Send + Sync + 'static,
    ) -> Self {
        self.on_field_repair = Some(std::sync::Arc::new(hook));
        self
    }

    /// build the client.
    ///
    /// # errors
    ///
    /// returns [`Error::InvalidResponse`] when the token, language or a custom
    /// header contains bytes that are not valid in an http header, or
    /// [`Error::Network`] when the default http client cannot be constructed.
    pub fn build(self) -> Result<Client> {
        let http = match self.http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .build()
                .map_err(|e| Error::Network(format!("could not build the http client: {e}")))?,
        };

        let transport = Transport::new(
            http,
            self.token.as_deref(),
            self.language.as_str(),
            &self.user_agent,
            &self.headers,
            self.timeout,
            self.download_timeout,
            self.retry,
            self.on_field_repair,
        )?;

        Ok(Client {
            inner: Arc::new(Inner {
                transport,
                base_url: self.base_url,
                language: self.language,
                device: self.device,
                token: self.token,
                account: RwLock::new(None),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_built_against_the_configured_origin() {
        let client = Client::builder()
            .base_url("https://example.test/")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "https://example.test");
        assert_eq!(client.url("/tracks/1"), "https://example.test/tracks/1");
    }

    #[test]
    fn an_anonymous_client_carries_no_token() {
        let client = Client::anonymous().unwrap();
        assert!(client.token().is_none());
        assert!(client.account_uid().is_none());
        assert!(client.me().is_none());
    }

    #[test]
    fn builder_options_land_on_the_client() {
        let client = Client::builder()
            .token("t")
            .language(Language::En)
            .device("os=Test")
            .build()
            .unwrap();
        assert_eq!(client.token(), Some("t"));
        assert_eq!(client.language(), Language::En);
        assert_eq!(client.device(), "os=Test");
    }

    #[test]
    fn a_uid_is_required_when_init_has_not_run() {
        let client = Client::anonymous().unwrap();
        assert!(client.resolve_uid(None).is_err());
        assert_eq!(client.resolve_uid(Some(42)).unwrap(), 42);
    }

    #[test]
    fn a_token_with_illegal_header_bytes_is_rejected_at_build_time() {
        // a newline in a header value would otherwise smuggle a second header
        assert!(Client::builder().token("bad\nvalue").build().is_err());
    }

    #[test]
    fn success_markers_cover_both_shapes_the_api_uses() {
        assert_eq!(
            Client::acknowledged(&Value::String("ok".into())).unwrap(),
            Mutation::default()
        );
        assert_eq!(
            Client::acknowledged(&serde_json::json!({"revision": 42}))
                .unwrap()
                .revision,
            Some(42)
        );
        assert_eq!(
            Client::acknowledged(&serde_json::json!({"status": "ok"})).unwrap(),
            Mutation::default()
        );
    }

    #[test]
    fn an_unacknowledged_mutation_is_an_error_rather_than_a_quiet_false() {
        for value in [
            Value::String("nope".into()),
            serde_json::json!({}),
            Value::Null,
            serde_json::json!({"status": "error"}),
            serde_json::json!([]),
        ] {
            let err = Client::acknowledged(&value).unwrap_err();
            assert!(matches!(err, Error::InvalidResponse(_)), "{value}: {err:?}");
            // the body is in the message, so the reason is not lost
            assert!(err.to_string().contains("did not acknowledge"), "{err}");
        }
    }

    #[test]
    fn a_revision_that_is_not_a_number_still_counts_as_acknowledged() {
        // the acknowledgement is the point; the revision is a bonus
        let mutation = Client::acknowledged(&serde_json::json!({"revision": "42"})).unwrap();
        assert_eq!(mutation.revision, None);
    }

    #[test]
    fn the_clip_endpoints_accept_any_object_they_answer_with() {
        // their success shape is not confirmed to be the siblings' bare "ok"
        assert!(Client::acknowledged_loosely(&serde_json::json!({"anything": 1})).is_ok());
        assert_eq!(
            Client::acknowledged_loosely(&serde_json::json!({"revision": 7}))
                .unwrap()
                .revision,
            Some(7)
        );
        // but a non-object is still a failure
        assert!(Client::acknowledged_loosely(&Value::Null).is_err());
        assert!(Client::acknowledged_loosely(&Value::String("nope".into())).is_err());
    }

    #[test]
    fn clones_share_the_account_cache() {
        let client = Client::anonymous().unwrap();
        let clone = client.clone();
        // both views start empty and point at the same inner state
        assert!(client.me().is_none());
        assert!(clone.me().is_none());
        assert!(Arc::ptr_eq(&client.inner, &clone.inner));
    }
}
