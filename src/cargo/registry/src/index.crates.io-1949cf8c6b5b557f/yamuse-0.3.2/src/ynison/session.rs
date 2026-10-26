//! one ynison session: the redirect handshake, then the state stream.
//!
//! connecting takes two websockets. the first asks a redirector which host owns
//! this account's session and gets a single-use ticket; the second carries that
//! ticket to that host and streams player state. reconnection is not handled
//! here — [`Realtime`](super::Realtime) drives it.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{Message, protocol::WebSocketConfig};

use super::messages::{DeviceInfo, device_info_header, encode_subprotocol, update_full_state};
use super::state::{
    PlayerState, Redirect, now_ms, parse_error_frame, parse_redirect_frame, parse_state_frame,
};
use crate::error::{Error, Result};

const BASE_URL: &str = "wss://ynison.music.yandex.ru";
const REDIRECT_SERVICE: &str = "redirector.YnisonRedirectService/GetRedirectToYnison";
const STATE_SERVICE: &str = "ynison_state.YnisonStateService/PutYnisonState";

/// how often to ping. the server drops a silent socket, and the pong keeps
/// [`SessionHandle::last_activity_ms`] fresh so a watchdog can tell "idle player"
/// from "dead pipe".
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);

/// how long to wait for the redirect frame.
const REDIRECT_TIMEOUT: Duration = Duration::from_secs(10);

/// configuration for one session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// oauth token.
    pub token: String,
    /// device id. keep this stable across reconnects — see [`super::RealtimeConfig`].
    pub device_id: String,
    /// device identity to present.
    pub device: DeviceInfo,
}

/// liveness of a running session, shared with whoever is driving it.
#[derive(Debug, Clone)]
pub struct SessionHandle {
    last_activity: std::sync::Arc<std::sync::atomic::AtomicI64>,
}

impl SessionHandle {
    fn new() -> Self {
        Self {
            last_activity: std::sync::Arc::new(std::sync::atomic::AtomicI64::new(now_ms())),
        }
    }

    fn touch(&self) {
        self.last_activity
            .store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    }

    /// when the socket last showed any sign of life: a frame, a ping or a pong.
    ///
    /// judge liveness by this, not by state-frame arrival: ynison only pushes a
    /// frame when the state actually changes, so a paused player is legitimately
    /// silent for minutes while the connection is perfectly healthy.
    #[must_use]
    pub fn last_activity_ms(&self) -> i64 {
        self.last_activity
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// milliseconds since the socket last showed any sign of life.
    #[must_use]
    pub fn idle_ms(&self) -> i64 {
        now_ms() - self.last_activity_ms()
    }
}

/// build the websocket request: auth and origin as headers, the device bundle
/// smuggled through the subprotocol list.
fn build_request(
    url: &str,
    config: &SessionConfig,
    redirect: Option<&Redirect>,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>> {
    let mut request = url
        .into_client_request()
        .map_err(|e| Error::ynison(format!("invalid ynison url: {e}")))?;

    let bundle = device_info_header(
        &config.device_id,
        &config.device,
        redirect.map(|r| (r.ticket.as_str(), r.session_id.as_str())),
    );
    let protocols = format!("Bearer, v2, {}", encode_subprotocol(&bundle));

    let headers = request.headers_mut();
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://music.yandex.ru"),
    );
    let mut auth = HeaderValue::from_str(&format!("OAuth {}", config.token))
        .map_err(|_| Error::ynison("token contains bytes invalid in a header"))?;
    auth.set_sensitive(true);
    headers.insert("Authorization", auth);
    headers.insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str(&protocols)
            .map_err(|_| Error::ynison("device bundle contains bytes invalid in a header"))?,
    );

    Ok(request)
}

fn ws_config() -> WebSocketConfig {
    // state frames carry the whole queue, which for a long playlist runs past the
    // 16 mb default only in pathological cases — but a dropped connection there
    // would look like an unexplained disconnect, so raise the ceiling.
    WebSocketConfig::default().max_message_size(Some(32 << 20))
}

/// ask the redirector which host owns this account's session.
async fn resolve_redirect(config: &SessionConfig) -> Result<Redirect> {
    let url = format!("{BASE_URL}/{REDIRECT_SERVICE}");
    let request = build_request(&url, config, None)?;

    let (mut socket, _) =
        tokio_tungstenite::connect_async_with_config(request, Some(ws_config()), false)
            .await
            .map_err(|e| Error::ynison(format!("redirect handshake failed: {e}")))?;

    let frame = tokio::time::timeout(REDIRECT_TIMEOUT, async {
        while let Some(message) = socket.next().await {
            match message {
                Ok(Message::Text(text)) => return Some(Ok(text.to_string())),
                Ok(Message::Close(_)) => {
                    return Some(Err(Error::ynison("redirect socket closed before a frame")));
                },
                // ping, pong and binary frames carry nothing we need here
                Ok(_) => {},
                Err(e) => return Some(Err(Error::ynison(format!("redirect socket error: {e}")))),
            }
        }
        None
    })
    .await
    .map_err(|_| Error::ynison("the redirect frame did not arrive in time"))?
    .ok_or_else(|| Error::ynison("redirect socket ended without a frame"))??;

    let _ = socket.close(None).await;

    if let Some(error) = parse_error_frame(&frame) {
        return Err(Error::Ynison {
            message: error.message,
            retry_after: error.go_away,
        });
    }
    parse_redirect_frame(&frame)
}

/// run one full session, forwarding every state frame to `states`.
///
/// resolves when the socket closes cleanly, and errors on a handshake or
/// transport failure. a server "go away" comes back as [`Error::Ynison`] with
/// [`Error::retry_after`] set.
pub async fn run(
    config: &SessionConfig,
    states: mpsc::Sender<PlayerState>,
    handle: SessionHandle,
) -> Result<()> {
    let redirect = resolve_redirect(config).await?;
    let url = format!("wss://{}/{STATE_SERVICE}", redirect.host);
    let request = build_request(&url, config, Some(&redirect))?;

    let (mut socket, _) =
        tokio_tungstenite::connect_async_with_config(request, Some(ws_config()), false)
            .await
            .map_err(|e| Error::ynison(format!("state handshake failed: {e}")))?;
    handle.touch();

    // register as a controller with an empty queue, or the server sends nothing.
    let hello = update_full_state(&config.device_id, &config.device).to_string();
    socket
        .send(Message::Text(hello.into()))
        .await
        .map_err(|e| Error::ynison(format!("could not send the initial state: {e}")))?;

    let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
    keepalive.tick().await; // the first tick fires immediately; skip it

    loop {
        tokio::select! {
            _ = keepalive.tick() => {
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                    return Err(Error::ynison("the state socket dropped while pinging"));
                }
            },
            message = socket.next() => {
                let Some(message) = message else {
                    return Ok(()); // stream ended
                };
                handle.touch();

                match message {
                    Ok(Message::Text(text)) => {
                        if let Some(error) = parse_error_frame(&text) {
                            return Err(Error::Ynison {
                                message: error.message,
                                retry_after: error.go_away,
                            });
                        }
                        // a frame we cannot parse is skipped, not fatal: one bad
                        // frame must not tear down a healthy session.
                        if let Ok(state) = parse_state_frame(&text) {
                            // a closed receiver means nobody is listening any more
                            if states.send(state).await.is_err() {
                                return Ok(());
                            }
                        }
                    },
                    Ok(Message::Close(_)) => return Ok(()),
                    Ok(_) => {}, // ping, pong and binary frames only prove liveness
                    Err(e) => return Err(Error::ynison(format!("state socket error: {e}"))),
                }
            },
        }
    }
}

/// a fresh liveness handle, for a caller about to [`run`] a session.
#[must_use]
pub fn new_handle() -> SessionHandle {
    SessionHandle::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SessionConfig {
        SessionConfig {
            token: "test-token".into(),
            device_id: "dev123".into(),
            device: DeviceInfo::ANDROID,
        }
    }

    #[test]
    fn the_request_carries_auth_origin_and_the_device_bundle() {
        let request = build_request("wss://example.test/svc", &config(), None).unwrap();
        let headers = request.headers();

        assert_eq!(headers["Origin"], "https://music.yandex.ru");
        assert_eq!(headers["Authorization"], "OAuth test-token");

        let protocols = headers["Sec-WebSocket-Protocol"].to_str().unwrap();
        assert!(protocols.starts_with("Bearer, v2, "));
        // the bundle must be percent-encoded, never raw json
        assert!(
            protocols.contains("%7B"),
            "the bundle should be encoded: {protocols}"
        );
        assert!(!protocols.contains('{'));
    }

    #[test]
    fn the_authorization_header_is_marked_sensitive() {
        // so a header dump in a log or panic does not leak the token
        let request = build_request("wss://example.test/svc", &config(), None).unwrap();
        assert!(request.headers()["Authorization"].is_sensitive());
    }

    #[test]
    fn the_second_leg_includes_the_redirect_ticket() {
        let redirect = Redirect {
            host: "state-host".into(),
            ticket: "tkt".into(),
            session_id: "sess".into(),
        };
        let request = build_request("wss://state-host/svc", &config(), Some(&redirect)).unwrap();
        let protocols = request.headers()["Sec-WebSocket-Protocol"]
            .to_str()
            .unwrap();

        // decode enough to confirm the ticket travelled
        assert!(
            protocols.contains("Ynison-Redirect-Ticket")
                || protocols.contains("%22Ynison-Redirect-Ticket%22")
        );
    }

    #[test]
    fn a_token_with_illegal_header_bytes_is_rejected() {
        let bad = SessionConfig {
            token: "bad\nvalue".into(),
            ..config()
        };
        assert!(build_request("wss://example.test/svc", &bad, None).is_err());
    }

    #[test]
    fn an_invalid_url_is_rejected() {
        assert!(build_request("not a url", &config(), None).is_err());
    }

    #[test]
    fn a_fresh_handle_starts_idle_at_roughly_zero() {
        let handle = new_handle();
        assert!(handle.idle_ms() < 1000);
        assert!(handle.last_activity_ms() > 0);
    }

    #[test]
    fn touching_a_handle_resets_its_idle_time() {
        let handle = new_handle();
        let before = handle.last_activity_ms();
        std::thread::sleep(Duration::from_millis(5));
        handle.touch();
        assert!(handle.last_activity_ms() >= before);
    }

    #[test]
    fn handles_share_state_across_clones() {
        let handle = new_handle();
        let clone = handle.clone();
        clone.touch();
        assert_eq!(handle.last_activity_ms(), clone.last_activity_ms());
    }
}
