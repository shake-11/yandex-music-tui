//! a typed, async client for the yandex music api.
//!
//! covers the catalogue (tracks, albums, artists, playlists), the library
//! (likes, dislikes, pins, pre-saves), radio, search, listening history,
//! lossless downloads and the ynison realtime player stream.
//!
//! # quick start
//!
//! ```no_run
//! use yamuse::Client;
//!
//! # async fn run() -> Result<(), yamuse::Error> {
//! let client = Client::builder().token(std::env::var("YM_TOKEN").unwrap()).build()?;
//! client.init().await?;
//!
//! let results = client.search("nine inch nails", Default::default()).await?;
//! for track in results.track_results() {
//!     println!("{} — {}", track.artists_name(), track.title.as_deref().unwrap_or("?"));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # authentication
//!
//! every catalogue call needs an oauth token. [`Client::device_auth`] runs the
//! full device flow — show a code, poll until the user confirms — so no browser
//! needs embedding:
//!
//! ```no_run
//! use yamuse::Client;
//!
//! # async fn run() -> Result<(), yamuse::Error> {
//! let client = Client::anonymous()?;
//! let token = client
//!     .device_auth(Default::default(), |code| {
//!         println!("go to {} and enter {}", code.verification_url.as_deref().unwrap_or(""), code.user_code.as_deref().unwrap_or(""));
//!     })
//!     .await?;
//! println!("token: {}", token.access_token.as_deref().unwrap_or(""));
//! # Ok(())
//! # }
//! ```
//!
//! # downloads
//!
//! two paths exist, and they are not interchangeable. the legacy one
//! ([`Client::track_download_info`]) serves mp3 and aac. the modern one
//! ([`Client::track_lossless_info`]) serves flac as an aes-ctr-encrypted
//! `encraw` stream — [`Downloader`](download::Downloader) decrypts it as it
//! writes, so a 300 mb file never lands in memory.
//!
//! ```no_run
//! # #[cfg(feature = "download")]
//! # async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
//! use yamuse::download::Downloader;
//!
//! let info = client.track_lossless_info(42, "lossless").await?;
//! Downloader::new(client.transport(), info.links())
//!     .maybe_encraw_key(info.key.clone())
//!     .to_file("track.flac")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # feature flags
//!
//! - `rustls-tls` *(default)* — tls via rustls.
//! - `native-tls` — tls via the platform library instead.
//! - `download` *(default)* — stream downloads to disk.
//! - `realtime` — the ynison player stream over websocket.
//! - `tracing` — emit `tracing` events for requests, retries, rate limits and
//!   ynison reconnects. without it the logging macros expand to nothing.
//! - `full` — `download` + `realtime` + `tracing`.
//!
//! # resilience
//!
//! this wraps a private api that changes without notice, so both layers degrade
//! rather than break: every model field is optional and unknown fields are
//! ignored, and polymorphic payloads whose type is unrecognized are preserved as
//! raw json instead of being dropped.
//!
//! idempotent `GET`s retry transient failures with jittered backoff, and a 429
//! with a `retry-after` waits out what the server asked for, up to a cap.
//! mutations never retry — a replayed `playlists/create` makes two playlists —
//! and a mutation the api does not acknowledge is an error rather than a `false`
//! nobody checks.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod log;

pub mod client;
pub mod crypto;
pub mod difference;
pub mod drift;
pub mod error;
pub mod models;
pub mod params;
pub mod sign;
pub mod tolerant;
pub mod transport;
pub mod types;

#[cfg(feature = "download")]
#[cfg_attr(docsrs, doc(cfg(feature = "download")))]
pub mod download;

#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub mod ynison;

mod api;
mod time;

/// the readme's rust blocks, compiled as doctests so they cannot rot.
///
/// `cfg(doctest)` keeps the readme out of the rendered documentation — this only
/// exists to make `cargo test` typecheck what the readme claims.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;

/// the same, for the russian readme. it carries the same code, and a translation
/// that silently stops compiling is worse than no translation.
#[cfg(doctest)]
#[doc = include_str!("../README_ru.md")]
struct ReadmeRuExamples;

pub use client::{BATCH_CHUNK, Client, ClientBuilder, DEFAULT_BASE_URL, DEFAULT_DEVICE};
pub use difference::{DiffTrack, Difference};
pub use error::{Error, Result};
pub use params::Params;
pub use transport::{RetryPolicy, Transport};

// listed rather than globbed: a glob at the crate root would re-export whatever
// `types` gains next, so a new enum could collide with an existing root name
// without anyone touching this file.
pub use types::{
    Dislikeable, Diversity, Language, Likeable, LyricsFormat, MoodEnergy, RotorFeedback,
    SearchType, SortBy, SortOrder, StationLanguage, StationType, Visibility,
};

// the option structs each method group defines live next to their methods; they
// are re-exported here so callers need only `use yamuse::*`.
pub use api::{
    ArtistAlbumsQuery, DeviceAuthOptions, HistoryQuery, MetatagListQuery, MetatagQuery,
    PlayAudioReport, RotorFeedbackReport, SearchQuery,
};
