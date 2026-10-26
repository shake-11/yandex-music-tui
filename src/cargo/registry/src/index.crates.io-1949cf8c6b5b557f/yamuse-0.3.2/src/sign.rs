//! request signing.
//!
//! two endpoints authenticate with an hmac over request parameters rather than
//! the oauth token alone: `/tracks/{id}/lyrics` and `/get-file-info`. both keys
//! and both message layouts come from the official android client.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// signing key extracted from the official android application.
pub const DEFAULT_SIGN_KEY: &str = "p93jhgh689SBReK6ghtw62";

/// codecs requested from `/get-file-info`, in the order the android client sends them.
pub const FILE_INFO_CODECS: &str = "flac,flac-mp4,mp3,aac,he-aac,aac-mp4,he-aac-mp4";

/// transport requested from `/get-file-info`. `encraw` returns an aes-ctr-encrypted stream.
pub const FILE_INFO_TRANSPORT: &str = "encraw";

/// a signature for a track-scoped request (lyrics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign {
    /// unix timestamp (seconds) the signature was created at.
    pub timestamp: i64,
    /// base64-encoded hmac-sha256 signature value.
    pub value: String,
}

/// a signature for a `/get-file-info` (lossless download) request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfoSign {
    /// unix timestamp (seconds) the signature was created at.
    pub ts: i64,
    /// base64-encoded hmac-sha256 signature value, with the trailing character dropped.
    pub value: String,
}

/// normalize a track identifier to its numeric form.
///
/// identifiers arrive either bare (`"42"`) or compound (`"42:1001"`, track and
/// album); both collapse to the numeric track id.
///
/// # examples
///
/// ```
/// use yamuse::sign::track_id_to_number;
///
/// assert_eq!(track_id_to_number("42"), Some(42));
/// assert_eq!(track_id_to_number("42:1001"), Some(42));
/// assert_eq!(track_id_to_number("nope"), None);
/// ```
#[must_use]
pub fn track_id_to_number(track_id: &str) -> Option<i64> {
    track_id.split(':').next()?.trim().parse().ok()
}

fn hmac_sha256(key: &str, message: &str) -> Vec<u8> {
    // hmac accepts a key of any length, so this never fails.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("hmac accepts keys of any length");
    mac.update(message.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// sign a track-scoped request as `hmac-sha256(key, "{track_id}{timestamp}")`.
///
/// pass the returned `timestamp` as `timeStamp` and `value` as `sign` in the query.
#[must_use]
pub fn sign_track(track_id: i64) -> Sign {
    sign_track_at(track_id, crate::time::unix_secs(), DEFAULT_SIGN_KEY)
}

/// [`sign_track`] with an explicit timestamp and key — the deterministic core,
/// exposed so the scheme can be verified against known vectors.
#[must_use]
pub fn sign_track_at(track_id: i64, timestamp: i64, key: &str) -> Sign {
    let value = BASE64.encode(hmac_sha256(key, &format!("{track_id}{timestamp}")));
    Sign { timestamp, value }
}

/// sign a `/get-file-info` request.
///
/// the message is `"{ts}{track_id}{quality}{codecs}{transport}"` with every comma
/// stripped; the base64 digest then loses its trailing character. the same
/// `codecs`/`transport` strings must be sent in the query or the signature will
/// not validate.
#[must_use]
pub fn sign_file_info(track_id: i64, quality: &str) -> FileInfoSign {
    sign_file_info_with(
        track_id,
        quality,
        FILE_INFO_CODECS,
        FILE_INFO_TRANSPORT,
        crate::time::unix_secs(),
        DEFAULT_SIGN_KEY,
    )
}

/// [`sign_file_info`] with every input explicit — the deterministic core.
#[must_use]
pub fn sign_file_info_with(
    track_id: i64,
    quality: &str,
    codecs: &str,
    transport: &str,
    ts: i64,
    key: &str,
) -> FileInfoSign {
    let message = format!("{ts}{track_id}{quality}{codecs}{transport}").replace(',', "");
    let mut value = BASE64.encode(hmac_sha256(key, &message));
    value.pop();
    FileInfoSign { ts, value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_ids_normalize_to_their_numeric_part() {
        assert_eq!(track_id_to_number("42"), Some(42));
        assert_eq!(track_id_to_number("42:1001"), Some(42));
        assert_eq!(track_id_to_number(" 42 "), Some(42));
        assert_eq!(track_id_to_number("-5"), Some(-5));
        assert_eq!(track_id_to_number(""), None);
        assert_eq!(track_id_to_number("abc"), None);
        assert_eq!(track_id_to_number("abc:1"), None);
    }

    // vectors computed from the reference implementation
    // (`createHmac('sha256', key).update(msg).digest('base64')`).
    #[test]
    fn track_signature_matches_the_reference_vector() {
        let sign = sign_track_at(42, 1_700_000_000, DEFAULT_SIGN_KEY);
        assert_eq!(sign.timestamp, 1_700_000_000);
        assert_eq!(sign.value, "xbCyTDmnI1xqeQil9/hP1qB2FtKWwefJgL9dAJueaYs=");
    }

    #[test]
    fn file_info_signature_matches_the_reference_vector() {
        let sign = sign_file_info_with(
            42,
            "lossless",
            FILE_INFO_CODECS,
            FILE_INFO_TRANSPORT,
            1_700_000_000,
            DEFAULT_SIGN_KEY,
        );
        assert_eq!(sign.ts, 1_700_000_000);
        // trailing base64 character dropped, as the android client does
        assert_eq!(sign.value, "5Xma89MBvxVhn4bCHIDRA/W8OGjHoibkEZj5b0DbRp8");
        assert_eq!(sign.value.len(), 43);
    }

    #[test]
    fn file_info_message_strips_commas_from_the_codec_list() {
        // a codec list differing only in commas must produce the same signature
        let with_commas =
            sign_file_info_with(1, "lossless", "flac,mp3", "encraw", 100, DEFAULT_SIGN_KEY);
        let without =
            sign_file_info_with(1, "lossless", "flacmp3", "encraw", 100, DEFAULT_SIGN_KEY);
        assert_eq!(with_commas.value, without.value);
    }

    #[test]
    fn signatures_change_with_every_input() {
        let base = sign_file_info_with(1, "lossless", "flac", "encraw", 100, DEFAULT_SIGN_KEY);
        for other in [
            sign_file_info_with(2, "lossless", "flac", "encraw", 100, DEFAULT_SIGN_KEY),
            sign_file_info_with(1, "normal", "flac", "encraw", 100, DEFAULT_SIGN_KEY),
            sign_file_info_with(1, "lossless", "mp3", "encraw", 100, DEFAULT_SIGN_KEY),
            sign_file_info_with(1, "lossless", "flac", "raw", 100, DEFAULT_SIGN_KEY),
            sign_file_info_with(1, "lossless", "flac", "encraw", 101, DEFAULT_SIGN_KEY),
            sign_file_info_with(1, "lossless", "flac", "encraw", 100, "other-key"),
        ] {
            assert_ne!(base.value, other.value);
        }
    }

    #[test]
    fn helpers_stamp_the_current_time() {
        let now = crate::time::unix_secs();
        assert!(sign_track(1).timestamp >= now);
        assert!(sign_file_info(1, "lossless").ts >= now);
    }
}
