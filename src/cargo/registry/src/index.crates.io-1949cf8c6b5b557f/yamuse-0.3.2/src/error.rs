//! error type raised by every fallible operation in the crate.
//!
//! all failures funnel into one [`Error`] enum, so a single `match` covers the
//! whole surface. the http-status variants mirror what the api reports:
//! `unauthorized` for 401/403, `bad_request` for 400, and so on.

use std::io;

/// convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// every failure this crate can produce.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// authentication or authorization failed (http 401/403, invalid token).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// the server rejected the request as malformed (http 400).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// the requested resource does not exist (http 404).
    #[error("not found: {0}")]
    NotFound(String),

    /// the request conflicts with the resource's current state (http 409).
    ///
    /// deterministic — retrying an identical request fails the same way.
    #[error("conflict: {0}")]
    Conflict(String),

    /// the request payload exceeds the server's size limit (http 413).
    ///
    /// deterministic — retrying an identical request fails the same way.
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),

    /// the server is rate limiting this client (http 429).
    ///
    /// unlike the other 4xx variants this one *is* retryable: it says "not now",
    /// not "not ever". [`retry_after`](Self::retry_after) carries how long the
    /// server asked for, when it said.
    #[error("rate limited: {message}")]
    RateLimited {
        /// the message extracted from the response body.
        message: String,
        /// how long the server asked us to wait, from its `retry-after` header.
        retry_after: Option<std::time::Duration>,
    },

    /// any other non-success http status.
    #[error("http {status}: {message}")]
    Http {
        /// the status code returned by the server.
        status: u16,
        /// the message extracted from the response body.
        message: String,
    },

    /// the transport failed (connection reset, dns, tls, ...).
    #[error("network: {0}")]
    Network(String),

    /// the request exceeded its timeout.
    #[error("timed out")]
    TimedOut,

    /// the response body was not the json this crate expected.
    #[error("invalid server response: {0}")]
    InvalidResponse(String),

    /// an operation needed an identifier that was not supplied.
    #[error("id missing: {0}")]
    IdMissing(&'static str),

    /// a track was requested with a bitrate or codec that is not available.
    #[error("invalid bitrate: {0}")]
    InvalidBitrate(String),

    /// the oauth device flow failed (excluding the expected pending state).
    #[error("device auth: {0}")]
    DeviceAuth(String),

    /// the ynison realtime protocol failed.
    #[error("ynison: {message}")]
    Ynison {
        /// what went wrong.
        message: String,
        /// how long the server asked us to wait before reconnecting, when it
        /// sent a "go away" frame.
        retry_after: Option<std::time::Duration>,
    },

    /// a filesystem operation failed while streaming a download to disk.
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

impl Error {
    /// map an http status and the message extracted from its body to a variant.
    ///
    /// `retry_after` is what the response's `retry-after` header asked for, when
    /// it sent one; only 429 has somewhere to carry it.
    pub(crate) fn from_status(
        status: u16,
        message: String,
        retry_after: Option<std::time::Duration>,
    ) -> Self {
        match status {
            401 | 403 => Self::Unauthorized(message),
            400 => Self::BadRequest(message),
            404 => Self::NotFound(message),
            409 => Self::Conflict(message),
            413 => Self::PayloadTooLarge(message),
            429 => Self::RateLimited {
                message,
                retry_after,
            },
            502 => Self::Network("bad gateway".into()),
            _ => Self::Http { status, message },
        }
    }

    /// build a ynison error without a server-requested delay.
    #[cfg(feature = "realtime")]
    pub(crate) fn ynison(message: impl Into<String>) -> Self {
        Self::Ynison {
            message: message.into(),
            retry_after: None,
        }
    }

    /// whether retrying an identical request could plausibly succeed.
    ///
    /// transport-level failures and 5xx qualify, and so does 429 — being rate
    /// limited means "not now", not "not ever". the other deterministic api
    /// rejections (4xx) always fail the same way, so retrying them just burns
    /// quota.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::TimedOut | Self::Network(_) | Self::RateLimited { .. } => true,
            Self::Http { status, .. } => *status >= 500,
            _ => false,
        }
    }

    /// how long the server asked us to wait before trying again, if it did.
    ///
    /// set by a `retry-after` on a 429, and by a ynison "go away" frame. treat it
    /// as a hint with a ceiling rather than an instruction: this api answers
    /// "temporarily unavailable" with delays of an hour and more.
    #[must_use]
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            Self::Ynison { retry_after, .. } | Self::RateLimited { retry_after, .. } => {
                *retry_after
            },
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::TimedOut;
        }
        if err.is_decode() {
            return Self::InvalidResponse(err.to_string());
        }
        Self::Network(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidResponse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_status(status: u16) -> Error {
        Error::from_status(status, String::new(), None)
    }

    #[test]
    fn status_maps_to_the_matching_variant() {
        assert!(matches!(from_status(401), Error::Unauthorized(_)));
        assert!(matches!(from_status(403), Error::Unauthorized(_)));
        assert!(matches!(from_status(400), Error::BadRequest(_)));
        assert!(matches!(from_status(404), Error::NotFound(_)));
        assert!(matches!(from_status(409), Error::Conflict(_)));
        assert!(matches!(from_status(413), Error::PayloadTooLarge(_)));
        assert!(matches!(from_status(429), Error::RateLimited { .. }));
        assert!(matches!(from_status(502), Error::Network(_)));
        assert!(matches!(from_status(418), Error::Http { status: 418, .. }));
    }

    #[test]
    fn a_rate_limit_carries_the_delay_the_server_asked_for() {
        let limited = Error::from_status(
            429,
            "too many requests".into(),
            Some(std::time::Duration::from_secs(7)),
        );
        assert_eq!(
            limited.retry_after(),
            Some(std::time::Duration::from_secs(7))
        );
        assert!(limited.is_retryable());

        // a 429 without the header is still retryable, just without a hint
        assert_eq!(from_status(429).retry_after(), None);
        assert!(from_status(429).is_retryable());
    }

    #[test]
    fn only_transport_failures_are_retryable() {
        assert!(Error::TimedOut.is_retryable());
        assert!(Error::Network("reset".into()).is_retryable());
        assert!(
            Error::Http {
                status: 503,
                message: String::new()
            }
            .is_retryable()
        );

        assert!(!Error::BadRequest(String::new()).is_retryable());
        assert!(!Error::NotFound(String::new()).is_retryable());
        assert!(!Error::Conflict(String::new()).is_retryable());
        assert!(!Error::PayloadTooLarge(String::new()).is_retryable());
        assert!(!Error::Unauthorized(String::new()).is_retryable());
        assert!(!Error::IdMissing("uid").is_retryable());
        assert!(
            !Error::Http {
                status: 418,
                message: String::new()
            }
            .is_retryable()
        );
    }

    #[test]
    fn retry_after_is_carried_only_where_the_server_sent_one() {
        let go_away = Error::Ynison {
            message: "temporary unavailable".into(),
            retry_after: Some(std::time::Duration::from_secs(30)),
        };
        assert_eq!(
            go_away.retry_after(),
            Some(std::time::Duration::from_secs(30))
        );

        let plain = Error::Ynison {
            message: "boom".into(),
            retry_after: None,
        };
        assert_eq!(plain.retry_after(), None);
        assert_eq!(Error::TimedOut.retry_after(), None);
    }
}
