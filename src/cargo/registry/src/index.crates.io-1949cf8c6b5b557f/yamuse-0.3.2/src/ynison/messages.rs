//! outgoing ynison requests and device identity.
//!
//! the protocol is betterproto's `to_json()` shape: camelCase keys, int64 values
//! as strings, and default values (`0`, `""`, `false`, enum `0`) omitted. the
//! literals below mirror that exactly — adding an "obvious" default back in makes
//! the server reject the frame.

use rand::Rng;
use serde_json::{Value, json};

/// device identity presented to ynison.
///
/// presenting as a known app makes the server expose more state fields than it
/// does for an anonymous web client — [`DeviceInfo::ANDROID`] is the richest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// device type for the request body: `WEB`, `ANDROID`, ...
    pub r#type: &'static str,
    /// numeric code for the `Ynison-Device-Info` header: `1` web, `2` android.
    pub header_type: &'static str,
    /// app name.
    pub app_name: &'static str,
    /// app version.
    pub app_version: Option<&'static str>,
    /// device title.
    pub title: &'static str,
}

impl DeviceInfo {
    /// an android client identity, which unlocks the richest state payload.
    pub const ANDROID: Self = Self {
        r#type: "ANDROID",
        header_type: "2",
        app_name: "ru.yandex.music",
        app_version: Some("2026.05.3"),
        title: "yamuse",
    };

    /// a generic web sdk identity.
    pub const WEB: Self = Self {
        r#type: "WEB",
        header_type: "1",
        app_name: "yamuse",
        app_version: None,
        title: "yamuse",
    };
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self::ANDROID
    }
}

/// generate a random device id: 8 cryptographically-random bytes as hex.
#[must_use]
pub fn generate_device_id() -> String {
    let bytes: [u8; 8] = rand::rng().random();
    hex::encode(bytes)
}

/// generate a request id — a uuid v4.
#[must_use]
pub fn generate_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// build the request that registers this device as a remote controller with an
/// empty queue. sent immediately after the state socket opens.
///
/// registering as a *controller* rather than a player is what makes the server
/// stream someone else's playback state to us instead of expecting us to produce
/// it.
#[must_use]
pub fn update_full_state(device_id: &str, device: &DeviceInfo) -> Value {
    let mut info = json!({
        "deviceId": device_id,
        "title": device.title,
        "type": device.r#type,
        "appName": device.app_name,
    });
    if let Some(version) = device.app_version {
        info["appVersion"] = json!(version);
    }

    json!({
        "updateFullState": {
            "playerState": {
                "status": {
                    "paused": true,
                    "playbackSpeed": 1,
                    "version": { "deviceId": device_id },
                },
                "playerQueue": {
                    "entityType": "VARIOUS",
                    "currentPlayableIndex": -1,
                    "options": { "repeatMode": "NONE" },
                    "version": { "deviceId": device_id },
                    "fromOptional": "",
                },
            },
            "device": {
                "info": info,
                "capabilities": { "canBeRemoteController": true },
                "volumeInfo": {},
            },
        },
        "rid": generate_request_id(),
    })
}

/// the `Ynison-Device-Info` bundle, sent as a websocket subprotocol.
///
/// the handshake has nowhere else to put it: browsers cannot set arbitrary
/// headers on a websocket, so ynison smuggles this json through
/// `Sec-WebSocket-Protocol`, percent-encoded.
#[must_use]
pub fn device_info_header(
    device_id: &str,
    device: &DeviceInfo,
    redirect: Option<(&str, &str)>,
) -> String {
    let inner = json!({ "app_name": device.app_name, "type": device.header_type });
    let mut bundle = json!({
        "Ynison-Device-Id": device_id,
        "Ynison-Device-Info": inner.to_string(),
    });
    if let Some((ticket, session_id)) = redirect {
        bundle["Ynison-Redirect-Ticket"] = json!(ticket);
        bundle["Ynison-Session-Id"] = json!(session_id);
    }
    bundle.to_string()
}

/// percent-encode a subprotocol value.
///
/// `Sec-WebSocket-Protocol` is a token list, so the json bundle has to be encoded
/// the way `encodeURIComponent` would.
#[must_use]
pub fn encode_subprotocol(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => (byte as char).to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_ids_are_sixteen_hex_characters() {
        let id = generate_device_id();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn device_ids_differ_between_calls() {
        let ids: std::collections::HashSet<String> =
            (0..20).map(|_| generate_device_id()).collect();
        assert!(ids.len() > 15);
    }

    #[test]
    fn request_ids_are_uuids() {
        let id = generate_request_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.matches('-').count(), 4);
    }

    #[test]
    fn the_full_state_request_registers_as_a_remote_controller() {
        let request = update_full_state("abc123", &DeviceInfo::ANDROID);
        let full = &request["updateFullState"];

        assert_eq!(
            full["device"]["capabilities"]["canBeRemoteController"],
            json!(true)
        );
        assert_eq!(full["device"]["info"]["deviceId"], json!("abc123"));
        assert_eq!(full["device"]["info"]["type"], json!("ANDROID"));
        assert_eq!(full["device"]["info"]["appName"], json!("ru.yandex.music"));
        assert_eq!(full["device"]["info"]["appVersion"], json!("2026.05.3"));

        // an empty queue: the controller declares it is not the one playing
        assert_eq!(
            full["playerState"]["playerQueue"]["currentPlayableIndex"],
            json!(-1)
        );
        assert_eq!(
            full["playerState"]["playerQueue"]["entityType"],
            json!("VARIOUS")
        );
        assert_eq!(full["playerState"]["status"]["paused"], json!(true));
        assert_eq!(
            full["playerState"]["status"]["version"]["deviceId"],
            json!("abc123")
        );
        assert!(request["rid"].is_string());
    }

    #[test]
    fn a_web_identity_omits_the_app_version_entirely() {
        // betterproto drops empty strings; sending `appVersion: ""` is not the same
        let request = update_full_state("id", &DeviceInfo::WEB);
        let info = &request["updateFullState"]["device"]["info"];
        assert_eq!(info["type"], json!("WEB"));
        assert!(info.get("appVersion").is_none());
    }

    #[test]
    fn the_device_header_omits_redirect_fields_on_the_first_leg() {
        let header = device_info_header("dev1", &DeviceInfo::ANDROID, None);
        let parsed: Value = serde_json::from_str(&header).unwrap();
        assert_eq!(parsed["Ynison-Device-Id"], json!("dev1"));
        assert!(parsed.get("Ynison-Redirect-Ticket").is_none());
        assert!(parsed.get("Ynison-Session-Id").is_none());

        // the nested info is a json *string*, not an object
        let inner: Value =
            serde_json::from_str(parsed["Ynison-Device-Info"].as_str().unwrap()).unwrap();
        assert_eq!(inner["app_name"], json!("ru.yandex.music"));
        assert_eq!(inner["type"], json!("2"));
    }

    #[test]
    fn the_device_header_carries_the_ticket_on_the_second_leg() {
        let header = device_info_header("dev1", &DeviceInfo::WEB, Some(("tkt", "sess")));
        let parsed: Value = serde_json::from_str(&header).unwrap();
        assert_eq!(parsed["Ynison-Redirect-Ticket"], json!("tkt"));
        assert_eq!(parsed["Ynison-Session-Id"], json!("sess"));
    }

    #[test]
    fn subprotocol_encoding_escapes_everything_a_token_cannot_hold() {
        assert_eq!(
            encode_subprotocol(r#"{"a":"b"}"#),
            "%7B%22a%22%3A%22b%22%7D"
        );
        assert_eq!(encode_subprotocol("plain-Text_1.0~"), "plain-Text_1.0~");
        assert_eq!(encode_subprotocol(" "), "%20");
    }

    #[test]
    fn android_is_the_default_identity() {
        assert_eq!(DeviceInfo::default(), DeviceInfo::ANDROID);
    }
}
