//! player-state frames.
//!
//! ynison speaks betterproto's json shape: default values are omitted entirely,
//! and keys arrive snake_case on the wire while the schema documents them
//! camelCase. both spellings are accepted here, and every absent field falls back
//! to a sane default rather than failing the frame — a parse error would drop a
//! whole state update.

use serde_json::Value;

use crate::error::{Error, Result};

/// the redirector response pointing at the state-service host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    /// state-service host to connect to.
    pub host: String,
    /// ticket authorizing the state connection.
    pub ticket: String,
    /// session id — an int64 the api sends as a string.
    pub session_id: String,
}

/// one entry of the playable queue.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Playable {
    /// the track id, when the entry carries one.
    pub playable_id: Option<String>,
}

/// a snapshot of player state.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PlayerState {
    /// the playable queue.
    pub playable_list: Vec<Playable>,
    /// index of the currently playing item, or `-1` when nothing is.
    pub current_playable_index: i64,
    /// id of the playing context: album, playlist, ...
    pub entity_id: Option<String>,
    /// type of the playing context.
    pub entity_type: Option<String>,
    /// whether playback is paused.
    pub paused: bool,
    /// track duration in milliseconds.
    pub duration_ms: i64,
    /// playback position in milliseconds, as of `timestamp_ms`.
    pub progress_ms: i64,
    /// playback speed multiplier.
    pub playback_speed: f64,
    /// server timestamp of the snapshot, in milliseconds since the epoch.
    pub timestamp_ms: i64,
}

impl PlayerState {
    /// the id of the track playing right now, when the index is in range.
    #[must_use]
    pub fn current_playable_id(&self) -> Option<&str> {
        let index = usize::try_from(self.current_playable_index).ok()?;
        self.playable_list.get(index)?.playable_id.as_deref()
    }

    /// the playback position extrapolated to this moment.
    ///
    /// ynison only pushes a frame when the state changes, so a frame that arrived
    /// thirty seconds ago still reports the position from then. this advances it by
    /// the elapsed wall-clock time, clamped to the track length.
    #[must_use]
    pub fn live_progress_ms(&self) -> i64 {
        if self.paused || self.timestamp_ms == 0 {
            return self.progress_ms.max(0);
        }
        let elapsed_ms = now_ms().saturating_sub(self.timestamp_ms);
        // both casts are exact for the values involved: a wall-clock delta in ms
        // stays far inside f64's 2^53 exact-integer range, and scaling it by a
        // playback speed of 0.5–3 cannot leave it.
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let elapsed = (elapsed_ms as f64 * self.playback_speed) as i64;
        let progress = self.progress_ms.saturating_add(elapsed);
        let ceiling = if self.duration_ms > 0 {
            self.duration_ms
        } else {
            progress
        };
        progress.clamp(0, ceiling)
    }
}

/// milliseconds since the unix epoch.
pub(crate) use crate::time::unix_millis as now_ms;

/// read the first present key out of several spellings.
fn field<'a>(object: Option<&'a Value>, names: &[&str]) -> Option<&'a Value> {
    let map = object?.as_object()?;
    names
        .iter()
        .filter_map(|name| map.get(*name))
        .find(|value| !value.is_null())
}

/// coerce a json number-or-string to i64, defaulting when neither.
fn as_i64(value: Option<&Value>, default: i64) -> i64 {
    match value {
        Some(Value::Number(n)) => n.as_i64().or_else(|| {
            // a millisecond timestamp only arrives as a float when the server
            // widened the field; truncating the fraction is the intent.
            #[allow(clippy::cast_possible_truncation)]
            n.as_f64().map(|f| f as i64)
        }),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
    .unwrap_or(default)
}

/// coerce a json number-or-string to f64, defaulting when neither.
fn as_f64(value: Option<&Value>, default: f64) -> f64 {
    match value {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
        Some(Value::String(s)) => s.parse().unwrap_or(default),
        _ => default,
    }
}

fn as_string(value: Option<&Value>) -> Option<String> {
    value?.as_str().map(std::borrow::ToOwned::to_owned)
}

/// parse a state frame.
///
/// # errors
///
/// returns [`Error::Ynison`] only when the frame is not json at all; a frame that
/// is json but missing fields parses into defaults.
pub fn parse_state_frame(message: &str) -> Result<PlayerState> {
    let root: Value = serde_json::from_str(message)
        .map_err(|e| Error::ynison(format!("state frame is not json: {e}")))?;

    let player_state = field(Some(&root), &["player_state", "playerState"]);
    let queue = field(player_state, &["player_queue", "playerQueue"]);
    let status = field(player_state, &["status"]);
    let version = field(status, &["version"]);

    let playable_list = field(queue, &["playable_list", "playableList"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| Playable {
                    playable_id: as_string(field(Some(item), &["playable_id", "playableId"])),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PlayerState {
        playable_list,
        current_playable_index: as_i64(
            field(queue, &["current_playable_index", "currentPlayableIndex"]),
            0,
        ),
        entity_id: as_string(field(queue, &["entity_id", "entityId"])),
        entity_type: as_string(field(queue, &["entity_type", "entityType"])),
        paused: field(status, &["paused"])
            .and_then(Value::as_bool)
            .unwrap_or(false),
        duration_ms: as_i64(field(status, &["duration_ms", "durationMs"]), 0),
        progress_ms: as_i64(field(status, &["progress_ms", "progressMs"]), 0),
        playback_speed: as_f64(field(status, &["playback_speed", "playbackSpeed"]), 1.0),
        timestamp_ms: as_i64(field(version, &["timestamp_ms", "timestampMs"]), 0),
    })
}

/// parse a redirect frame.
///
/// # errors
///
/// returns [`Error::Ynison`] when the frame is not json or omits a required field.
pub fn parse_redirect_frame(message: &str) -> Result<Redirect> {
    let root: Value = serde_json::from_str(message)
        .map_err(|e| Error::ynison(format!("redirect frame is not json: {e}")))?;

    let host = as_string(field(Some(&root), &["host"])).unwrap_or_default();
    let ticket =
        as_string(field(Some(&root), &["redirect_ticket", "redirectTicket"])).unwrap_or_default();
    // the session id is an int64; the api sends it quoted, but tolerate a number.
    let session_id = match field(Some(&root), &["session_id", "sessionId"]) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };

    if host.is_empty() || ticket.is_empty() || session_id.is_empty() {
        let preview: String = message.chars().take(200).collect();
        return Err(Error::ynison(format!(
            "incomplete redirect frame: {preview}"
        )));
    }
    Ok(Redirect {
        host,
        ticket,
        session_id,
    })
}

/// an error frame, plus the delay the server asked for.
#[derive(Debug, Clone)]
pub(crate) struct ErrorFrame {
    pub message: String,
    pub go_away: Option<std::time::Duration>,
}

/// parse an error frame, returning `None` when the frame is not one.
pub(crate) fn parse_error_frame(message: &str) -> Option<ErrorFrame> {
    let root: Value = serde_json::from_str(message).ok()?;
    let error = root.get("error")?;
    let text = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("ynison error")
        .to_owned();
    let go_away = error
        .get("details")
        .and_then(|d| d.get("ynison-go-away-for-seconds"))
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| v.as_f64())
        })
        .filter(|secs| *secs > 0.0)
        .map(std::time::Duration::from_secs_f64);

    Some(ErrorFrame {
        message: text,
        go_away,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snake_case_frame_parses() {
        let state = parse_state_frame(
            r#"{"player_state":{
                "player_queue":{
                    "playable_list":[{"playable_id":"111"},{"playable_id":"222"}],
                    "current_playable_index":1,
                    "entity_id":"1000","entity_type":"PLAYLIST"
                },
                "status":{"paused":false,"duration_ms":"210000","progress_ms":"30000",
                          "playback_speed":1,"version":{"timestamp_ms":"1700000000000"}}
            }}"#,
        )
        .unwrap();

        assert_eq!(state.playable_list.len(), 2);
        assert_eq!(state.current_playable_index, 1);
        assert_eq!(state.current_playable_id(), Some("222"));
        assert_eq!(state.entity_type.as_deref(), Some("PLAYLIST"));
        assert!(!state.paused);
        assert_eq!(state.duration_ms, 210_000);
        assert_eq!(state.progress_ms, 30_000);
        assert_eq!(state.timestamp_ms, 1_700_000_000_000);
    }

    #[test]
    fn a_camel_case_frame_parses_identically() {
        let state = parse_state_frame(
            r#"{"playerState":{"playerQueue":{"playableList":[{"playableId":"7"}],
                "currentPlayableIndex":0,"entityId":"9","entityType":"ALBUM"},
                "status":{"paused":true,"durationMs":1000,"progressMs":500,
                          "playbackSpeed":1.5,"version":{"timestampMs":42}}}}"#,
        )
        .unwrap();

        assert_eq!(state.current_playable_id(), Some("7"));
        assert_eq!(state.entity_type.as_deref(), Some("ALBUM"));
        assert!(state.paused);
        assert_eq!(state.duration_ms, 1000);
        assert!((state.playback_speed - 1.5).abs() < f64::EPSILON);
        assert_eq!(state.timestamp_ms, 42);
    }

    #[test]
    fn omitted_defaults_fall_back_rather_than_failing() {
        // betterproto drops every default value, so an idle player sends almost nothing
        let state = parse_state_frame("{}").unwrap();
        assert!(state.playable_list.is_empty());
        assert_eq!(state.current_playable_index, 0);
        assert!(!state.paused);
        assert_eq!(state.duration_ms, 0);
        assert!(
            (state.playback_speed - 1.0).abs() < f64::EPSILON,
            "speed defaults to 1, not 0"
        );
        assert_eq!(state.current_playable_id(), None);
    }

    #[test]
    fn a_non_json_frame_is_an_error() {
        assert!(parse_state_frame("not json").is_err());
    }

    #[test]
    fn an_out_of_range_index_yields_no_current_track() {
        let past_end = PlayerState {
            playable_list: vec![Playable {
                playable_id: Some("1".into()),
            }],
            current_playable_index: 5,
            ..Default::default()
        };
        assert_eq!(past_end.current_playable_id(), None);

        // -1 is what the api sends for "nothing playing"
        let nothing = PlayerState {
            current_playable_index: -1,
            ..Default::default()
        };
        assert_eq!(nothing.current_playable_id(), None);
    }

    #[test]
    fn live_progress_freezes_while_paused() {
        let paused = PlayerState {
            paused: true,
            progress_ms: 30_000,
            duration_ms: 210_000,
            timestamp_ms: now_ms() - 60_000,
            playback_speed: 1.0,
            ..Default::default()
        };
        assert_eq!(paused.live_progress_ms(), 30_000);
    }

    #[test]
    fn live_progress_advances_while_playing() {
        let playing = PlayerState {
            paused: false,
            progress_ms: 30_000,
            duration_ms: 210_000,
            timestamp_ms: now_ms() - 10_000,
            playback_speed: 1.0,
            ..Default::default()
        };
        let progress = playing.live_progress_ms();
        assert!(
            (39_000..=41_000).contains(&progress),
            "expected ~40s, got {progress}"
        );
    }

    #[test]
    fn live_progress_never_exceeds_the_track_length() {
        let overrun = PlayerState {
            paused: false,
            progress_ms: 200_000,
            duration_ms: 210_000,
            timestamp_ms: now_ms() - 600_000,
            playback_speed: 1.0,
            ..Default::default()
        };
        assert_eq!(overrun.live_progress_ms(), 210_000);
    }

    #[test]
    fn live_progress_honours_the_playback_speed() {
        let fast = PlayerState {
            paused: false,
            progress_ms: 0,
            duration_ms: 600_000,
            timestamp_ms: now_ms() - 10_000,
            playback_speed: 2.0,
            ..Default::default()
        };
        let progress = fast.live_progress_ms();
        assert!(
            (19_000..=21_000).contains(&progress),
            "expected ~20s, got {progress}"
        );
    }

    #[test]
    fn a_zero_timestamp_means_no_extrapolation_is_possible() {
        let unstamped = PlayerState {
            paused: false,
            progress_ms: 5_000,
            timestamp_ms: 0,
            playback_speed: 1.0,
            ..Default::default()
        };
        assert_eq!(unstamped.live_progress_ms(), 5_000);
    }

    #[test]
    fn a_complete_redirect_frame_parses() {
        let redirect = parse_redirect_frame(
            r#"{"host":"ynison-state.music.yandex.net","redirect_ticket":"tkt","session_id":"12345"}"#,
        )
        .unwrap();
        assert_eq!(redirect.host, "ynison-state.music.yandex.net");
        assert_eq!(redirect.ticket, "tkt");
        assert_eq!(redirect.session_id, "12345");
    }

    #[test]
    fn a_numeric_session_id_is_accepted() {
        let redirect =
            parse_redirect_frame(r#"{"host":"h","redirectTicket":"t","sessionId":12345}"#).unwrap();
        assert_eq!(redirect.session_id, "12345");
    }

    #[test]
    fn an_incomplete_redirect_frame_is_rejected() {
        assert!(parse_redirect_frame(r#"{"host":"h","redirect_ticket":"t"}"#).is_err());
        assert!(
            parse_redirect_frame(r#"{"host":"","redirect_ticket":"t","session_id":"1"}"#).is_err()
        );
        assert!(parse_redirect_frame("{}").is_err());
        assert!(parse_redirect_frame("nope").is_err());
    }

    #[test]
    fn error_frames_carry_the_go_away_delay() {
        let frame = parse_error_frame(
            r#"{"error":{"message":"temporary unavailable","details":{"ynison-go-away-for-seconds":"3600"}}}"#,
        )
        .unwrap();
        assert_eq!(frame.message, "temporary unavailable");
        assert_eq!(frame.go_away, Some(std::time::Duration::from_secs(3600)));
    }

    #[test]
    fn an_error_frame_without_details_still_parses() {
        let frame = parse_error_frame(r#"{"error":{"message":"boom"}}"#).unwrap();
        assert_eq!(frame.message, "boom");
        assert_eq!(frame.go_away, None);
    }

    #[test]
    fn a_state_frame_is_not_mistaken_for_an_error() {
        assert!(parse_error_frame(r#"{"player_state":{}}"#).is_none());
        assert!(parse_error_frame("not json").is_none());
    }

    #[test]
    fn a_non_positive_go_away_is_ignored() {
        let frame = parse_error_frame(
            r#"{"error":{"message":"x","details":{"ynison-go-away-for-seconds":"0"}}}"#,
        )
        .unwrap();
        assert_eq!(frame.go_away, None);
    }
}
