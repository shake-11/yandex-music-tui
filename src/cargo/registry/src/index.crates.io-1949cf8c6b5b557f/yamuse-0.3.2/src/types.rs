//! typed parameter enums.
//!
//! the api takes these as bare strings, and a mistyped one comes back as a
//! puzzling 400 rather than anything that names the problem. as real enums, a
//! typo is a compile error instead.

use std::fmt;

use serde::{Deserialize, Serialize};

/// render an enum as the wire string the api expects.
macro_rules! wire_display {
    ($name:ident) => {
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

/// response language.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// russian — the api default.
    #[default]
    Ru,
    /// english.
    En,
    /// ukrainian.
    Uk,
    /// uzbek.
    Uz,
    /// us english.
    Us,
    /// kazakh.
    Kk,
    /// armenian.
    Hy,
}

impl Language {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::Uk => "uk",
            Self::Uz => "uz",
            Self::Us => "us",
            Self::Kk => "kk",
            Self::Hy => "hy",
        }
    }
}
wire_display!(Language);

/// entity type to search for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    /// every entity type.
    #[default]
    All,
    /// artists only.
    Artist,
    /// users only.
    User,
    /// albums only.
    Album,
    /// playlists only.
    Playlist,
    /// tracks only.
    Track,
    /// podcasts only.
    Podcast,
    /// podcast episodes only.
    PodcastEpisode,
}

impl SearchType {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Artist => "artist",
            Self::User => "user",
            Self::Album => "album",
            Self::Playlist => "playlist",
            Self::Track => "track",
            Self::Podcast => "podcast",
            Self::PodcastEpisode => "podcast_episode",
        }
    }
}
wire_display!(SearchType);

/// lyrics format.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LyricsFormat {
    /// plain text.
    #[default]
    #[serde(rename = "TEXT")]
    Text,
    /// time-synced lrc.
    #[serde(rename = "LRC")]
    Lrc,
}

impl LyricsFormat {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Lrc => "LRC",
        }
    }
}
wire_display!(LyricsFormat);

/// visibility of a playlist.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// visible to everyone.
    #[default]
    Public,
    /// visible only to the owner.
    Private,
}

impl Visibility {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}
wire_display!(Visibility);

/// how a listing is sorted.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    /// by release year.
    #[default]
    Year,
    /// by rating.
    Rating,
}

impl SortBy {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Rating => "rating",
        }
    }
}
wire_display!(SortBy);

/// the direction of a sort.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// ascending.
    Asc,
    /// descending.
    #[default]
    Desc,
}

impl SortOrder {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}
wire_display!(SortOrder);

/// entity type that can be liked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Likeable {
    /// a track.
    Track,
    /// an artist.
    Artist,
    /// an album.
    Album,
    /// a playlist, identified as `ownerId:kind`.
    Playlist,
}

impl Likeable {
    /// the wire value, singular.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Playlist => "playlist",
        }
    }
}
wire_display!(Likeable);

/// entity type that can be disliked.
///
/// narrower than [`Likeable`] on purpose: the api only accepts dislikes for
/// tracks and artists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dislikeable {
    /// a track.
    Track,
    /// an artist.
    Artist,
}

impl Dislikeable {
    /// the wire value, singular.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Artist => "artist",
        }
    }
}
wire_display!(Dislikeable);

/// kind of feedback sent to the rotor while listening to a station.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotorFeedback {
    /// the radio session started.
    RadioStarted,
    /// a track started playing.
    TrackStarted,
    /// a track played to the end.
    TrackFinished,
    /// a track was skipped.
    Skip,
}

impl RotorFeedback {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RadioStarted => "radioStarted",
            Self::TrackStarted => "trackStarted",
            Self::TrackFinished => "trackFinished",
            Self::Skip => "skip",
        }
    }
}
wire_display!(RotorFeedback);

/// mood and energy preset for a station.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MoodEnergy {
    /// no preference.
    #[default]
    All,
    /// upbeat.
    Fun,
    /// energetic.
    Active,
    /// calm.
    Calm,
    /// melancholic.
    Sad,
}

impl MoodEnergy {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Fun => "fun",
            Self::Active => "active",
            Self::Calm => "calm",
            Self::Sad => "sad",
        }
    }
}
wire_display!(MoodEnergy);

/// track-diversity preset for a station.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Diversity {
    /// the api default mix.
    #[default]
    Default,
    /// favour tracks the listener already likes.
    Favorite,
    /// favour popular tracks.
    Popular,
    /// favour unfamiliar tracks.
    Discover,
}

impl Diversity {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Favorite => "favorite",
            Self::Popular => "popular",
            Self::Discover => "discover",
        }
    }
}
wire_display!(Diversity);

/// language preset for a station.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StationLanguage {
    /// any language.
    Any,
    /// russian only.
    Russian,
    /// everything but russian — the api's own default for this parameter.
    #[default]
    NotRussian,
}

impl StationLanguage {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Russian => "russian",
            Self::NotRussian => "not-russian",
        }
    }
}
wire_display!(StationLanguage);

/// station type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StationType {
    /// a regular rotor station.
    #[default]
    Rotor,
    /// a generative station.
    Generative,
}

impl StationType {
    /// the wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rotor => "rotor",
            Self::Generative => "generative",
        }
    }
}
wire_display!(StationType);

/// object kind addressable through the batch "list" endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchObject {
    Track,
    Album,
    Artist,
    Playlist,
}

impl BatchObject {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Album => "album",
            Self::Artist => "artist",
            Self::Playlist => "playlist",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_values_match_what_the_api_expects() {
        assert_eq!(SearchType::PodcastEpisode.as_str(), "podcast_episode");
        assert_eq!(SearchType::All.as_str(), "all");
        assert_eq!(LyricsFormat::Lrc.as_str(), "LRC");
        assert_eq!(LyricsFormat::Text.as_str(), "TEXT");
        assert_eq!(RotorFeedback::TrackFinished.as_str(), "trackFinished");
        assert_eq!(RotorFeedback::RadioStarted.as_str(), "radioStarted");
        assert_eq!(StationLanguage::NotRussian.as_str(), "not-russian");
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Language::Ru.as_str(), "ru");
        assert_eq!(SortBy::Year.as_str(), "year");
        assert_eq!(SortOrder::Desc.as_str(), "desc");
        assert_eq!(Likeable::Playlist.as_str(), "playlist");
        assert_eq!(Dislikeable::Artist.as_str(), "artist");
        assert_eq!(MoodEnergy::All.as_str(), "all");
        assert_eq!(Diversity::Default.as_str(), "default");
        assert_eq!(StationType::Generative.as_str(), "generative");
        assert_eq!(BatchObject::Playlist.as_str(), "playlist");
    }

    #[test]
    fn display_renders_the_wire_value() {
        assert_eq!(SearchType::Track.to_string(), "track");
        assert_eq!(StationLanguage::NotRussian.to_string(), "not-russian");
        assert_eq!(LyricsFormat::Lrc.to_string(), "LRC");
    }

    #[test]
    fn defaults_match_the_api_defaults() {
        assert_eq!(Language::default(), Language::Ru);
        assert_eq!(SearchType::default(), SearchType::All);
        assert_eq!(LyricsFormat::default(), LyricsFormat::Text);
        assert_eq!(Visibility::default(), Visibility::Public);
        assert_eq!(SortBy::default(), SortBy::Year);
        assert_eq!(SortOrder::default(), SortOrder::Desc);
        assert_eq!(MoodEnergy::default(), MoodEnergy::All);
        assert_eq!(Diversity::default(), Diversity::Default);
        assert_eq!(StationLanguage::default(), StationLanguage::NotRussian);
        assert_eq!(StationType::default(), StationType::Rotor);
    }
}
