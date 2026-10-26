//! optional `tracing` integration.
//!
//! wrapping a private api means the interesting failures are operational: a
//! mirror that went dark, a rate limit that started firing, a websocket that
//! reconnects every thirty seconds. `on_field_repair` reports drift in the
//! payloads, but says nothing about any of that.
//!
//! enabling the `tracing` feature turns the macros below into real events;
//! without it they expand to nothing at all, so the dependency stays optional and
//! the default build carries no logging machinery.
//!
//! **nothing here logs a url with its query attached.** the collective-join token
//! and the `get-file-info` signature both travel in the query, so call sites pass
//! [`redacted`] and log the path alone.

/// a url with its query string removed, for logging.
///
/// query parameters carry a signature on `get-file-info` and a share token on the
/// collective-playlist join, neither of which belongs in a log line.
#[cfg_attr(not(feature = "tracing"), allow(dead_code))]
pub(crate) fn redacted(url: &str) -> &str {
    match url.split_once('?') {
        Some((head, _)) => head,
        None => url,
    }
}

#[cfg(feature = "tracing")]
macro_rules! debug {
    ($($arg:tt)*) => { ::tracing::debug!($($arg)*) };
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

// not named `warn`: that collides with the built-in attribute of the same name
// the moment it is imported by path.
#[cfg(feature = "tracing")]
macro_rules! warning {
    ($($arg:tt)*) => { ::tracing::warn!($($arg)*) };
}

#[cfg(not(feature = "tracing"))]
macro_rules! warning {
    ($($arg:tt)*) => {};
}

pub(crate) use {debug, warning};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_string_never_survives_into_a_log_line() {
        // both of these really do carry a secret in the query
        assert_eq!(
            redacted("https://api.music.yandex.net/get-file-info?ts=1&sign=abc"),
            "https://api.music.yandex.net/get-file-info"
        );
        assert_eq!(
            redacted("https://api.music.yandex.net/playlists/collective/join?token=secret"),
            "https://api.music.yandex.net/playlists/collective/join"
        );
        assert_eq!(
            redacted("https://api.music.yandex.net/account/status"),
            "https://api.music.yandex.net/account/status"
        );
        assert_eq!(redacted("?only=query"), "");
    }
}
