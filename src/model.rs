//! Small, UI-friendly domain types.
//!
//! The `yamuse` models are faithful to a fast-moving private API: every field is
//! optional and some carry half-resolved references. The rest of the application
//! should not have to repeat that defensiveness, so API responses are normalised
//! here into display-ready values with stable identities.

use std::time::Duration;

/// The station identifier Yandex Music uses for a listener's personal wave.
pub const MY_WAVE_STATION: &str = "user:onyourwave";

/// One batch returned by the My Wave rotor.
#[derive(Debug, Clone, Default)]
pub struct WaveBatch {
    pub tracks: Vec<Track>,
    pub batch_id: Option<String>,
    pub radio_session_id: Option<String>,
    /// Last track id, passed back as the queue cursor for the next batch.
    pub last_track_id: Option<i64>,
}

/// A listening event reported back to the My Wave rotor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveFeedbackKind {
    RadioStarted,
    TrackStarted,
    TrackFinished,
    Skip,
}

/// A playable track with everything the interface needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub year: Option<i64>,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub explicit: bool,
    pub available: bool,
}

impl Track {
    /// Track duration as a `Duration`.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    /// A stable, filesystem-safe cache key.
    #[must_use]
    pub fn cache_key(&self) -> String {
        let mut key = String::with_capacity(self.id.len());
        for ch in self.id.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                key.push(ch);
            } else {
                key.push('_');
            }
        }
        if key.is_empty() {
            key.push_str("unknown");
        }
        key
    }
}

/// A playlist entry in the user's library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistSummary {
    pub uid: i64,
    pub kind: i64,
    pub title: String,
    pub owner: String,
    pub track_count: u32,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub uuid: Option<String>,
}

impl PlaylistSummary {
    /// Identity used by API calls and view matching.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.uid, self.kind)
    }
}

/// An artist in the user's library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistSummary {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
    pub track_count: Option<u32>,
}

/// An album in the user's library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumSummary {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub year: Option<i64>,
    pub track_count: u32,
    pub cover_url: Option<String>,
}

/// The signed-in listener.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Profile {
    pub login: String,
    pub display_name: String,
    pub has_plus: bool,
}

impl Profile {
    /// A compact name suitable for the header/footer.
    #[must_use]
    pub fn short_name(&self) -> &str {
        if !self.display_name.is_empty() {
            &self.display_name
        } else if !self.login.is_empty() {
            &self.login
        } else {
            "listener"
        }
    }
}

/// The library snapshot, fetched in parts so a single failing endpoint cannot
/// make the rest of the application unusable.
#[derive(Debug, Clone, Default)]
pub struct LibraryData {
    pub profile: Profile,
    pub liked_tracks: Vec<Track>,
    pub playlists: Vec<PlaylistSummary>,
    pub artists: Vec<ArtistSummary>,
    pub albums: Vec<AlbumSummary>,
    /// Human-readable problems from endpoints that failed, without aborting the
    /// whole load. Empty when everything succeeded.
    pub errors: Vec<String>,
}

/// A resolved, display-ready collection of tracks.
#[derive(Debug, Clone, Default)]
pub struct TrackCollection {
    pub title: String,
    pub subtitle: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub tracks: Vec<Track>,
}

/// Grouped catalogue search results.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub query: String,
    pub tracks: Vec<Track>,
    pub albums: Vec<AlbumSummary>,
    pub artists: Vec<ArtistSummary>,
    pub playlists: Vec<PlaylistSummary>,
}

impl SearchResults {
    /// Total number of actionable results.
    #[must_use]
    pub fn total(&self) -> usize {
        self.tracks.len() + self.albums.len() + self.artists.len() + self.playlists.len()
    }

    /// Whether the API returned anything worth rendering.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// Everything needed to replace a detailed view after a fetch.
#[derive(Debug, Clone, Default)]
pub struct LoadedCollection {
    pub title: String,
    pub subtitle: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub tracks: Vec<Track>,
}
