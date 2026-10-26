//! the album model — also how the api represents podcasts and audiobooks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::artist::Artist;
use super::misc::TrailerInfo;
use super::playlist::CustomWave;
use super::track::Track;
use super::{ContentRestrictions, Cover, CoverDerivedColors, Link, Pager};

/// a record label.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Label {
    /// label identifier.
    pub id: Option<i64>,
    /// label name.
    pub name: Option<String>,
    /// plain description.
    pub description: Option<String>,
    /// formatted (html) description.
    pub description_formatted: Option<String>,
    /// image uri template.
    pub image: Option<String>,
    /// external links.
    pub links: Vec<Link>,
    /// label kind.
    pub r#type: Option<String>,
}

/// a label as an album reports it.
///
/// most endpoints return a full object, a few return only the bare name — both
/// shapes land here rather than one of them being dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AlbumLabel {
    /// the full label object.
    Full(Label),
    /// only the label name.
    Name(String),
}

impl AlbumLabel {
    /// the label name, from whichever shape arrived.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Full(label) => label.name.as_deref(),
            Self::Name(name) => Some(name),
        }
    }
}

/// position of a track within an album: disc and index.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackPosition {
    /// volume (disc) number, one-based.
    pub volume: Option<i64>,
    /// track index within the volume, one-based.
    pub index: Option<i64>,
}

/// information about an album that has been deprecated or replaced.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Deprecation {
    /// identifier of the album that supersedes this one.
    pub target_album_id: Option<i64>,
    /// deprecation status.
    pub status: Option<String>,
    /// whether deprecation is complete.
    pub done: Option<bool>,
}

/// a call-to-action button shown on an album page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumActionButton {
    /// button caption.
    pub text: Option<String>,
    /// target url.
    pub url: Option<String>,
    /// button color.
    pub color: Option<String>,
}

/// a music album, also used for podcasts and audiobooks.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Album {
    /// unique album identifier.
    pub id: Option<i64>,
    /// error code, present when the album could not be resolved.
    pub error: Option<String>,
    /// album title.
    pub title: Option<String>,
    /// number of tracks.
    pub track_count: Option<i64>,
    /// album artists.
    pub artists: Vec<Artist>,
    /// record labels, as objects or bare names depending on the endpoint.
    pub labels: Vec<AlbumLabel>,
    /// whether the album is available.
    pub available: Option<bool>,
    /// whether the album is available to premium users.
    pub available_for_premium_users: Option<bool>,
    /// version, for example "deluxe edition".
    pub version: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// structured cover descriptor.
    pub cover: Option<Cover>,
    /// whether the album has a trailer.
    pub has_trailer: Option<bool>,
    /// options the album is available for.
    pub available_for_options: Vec<String>,
    /// whether the user finished listening, for audiobooks and podcasts.
    pub listening_finished: Option<bool>,
    /// regions where the album is available.
    pub available_regions: Vec<String>,
    /// metatag id the album is grouped under.
    pub meta_tag_id: Option<String>,
    /// sort order applied to the track list by the with-tracks endpoint.
    pub sort_order: Option<String>,
    /// background image uri template.
    pub background_image_url: Option<String>,
    /// whether the album is child-friendly content.
    pub child_content: Option<bool>,
    /// colors derived from the cover image.
    pub derived_colors: Option<CoverDerivedColors>,
    /// custom wave (radio) descriptor.
    pub custom_wave: Option<CustomWave>,
    /// duplicate albums — other editions of the same release.
    pub duplicates: Vec<Album>,
    /// pagination metadata for the track list.
    pub pager: Option<Pager>,
    /// album trailer, still free-form upstream.
    pub trailer: Option<Value>,
    /// explicit or content-warning marker.
    pub content_warning: Option<String>,
    /// genre.
    pub genre: Option<String>,
    /// text color suited for the cover.
    pub text_color: Option<String>,
    /// short description.
    pub short_description: Option<String>,
    /// full description.
    pub description: Option<String>,
    /// whether the album is a premiere.
    pub is_premiere: Option<bool>,
    /// whether the album is a banner feature.
    pub is_banner: Option<bool>,
    /// meta type, for example `music` or `podcast`.
    pub meta_type: Option<String>,
    /// storage directory.
    pub storage_dir: Option<String>,
    /// open graph image uri template.
    pub og_image: Option<String>,
    /// whether the album is recent.
    pub recent: Option<bool>,
    /// whether the album is flagged very important.
    pub very_important: Option<bool>,
    /// whether available on mobile.
    pub available_for_mobile: Option<bool>,
    /// whether only partially available.
    pub available_partially: Option<bool>,
    /// identifiers of the "best" tracks.
    pub bests: Vec<i64>,
    /// tracks grouped by volume (disc); present on the with-tracks endpoint.
    pub volumes: Vec<Vec<Track>>,
    /// release year.
    pub year: Option<i64>,
    /// release date, as iso 8601.
    pub release_date: Option<String>,
    /// album kind, for example `single` or `compilation`.
    pub r#type: Option<String>,
    /// album type marker, the newer key: `single`, `podcast`, `audiobook`, ...
    pub album_type: Option<String>,
    /// position of a single track within the album, in a contextual response.
    pub track_position: Option<TrackPosition>,
    /// regions where available.
    pub regions: Vec<String>,
    /// whether lyrics are available.
    pub lyrics_available: Option<bool>,
    /// total duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// explicit marker.
    pub explicit: Option<bool>,
    /// start date of availability.
    pub start_date: Option<String>,
    /// number of likes.
    pub likes_count: Option<i64>,
    /// deprecation and replacement info.
    pub deprecation: Option<Deprecation>,
    /// disclaimers.
    pub disclaimers: Vec<String>,
    /// call-to-action button.
    pub action_button: Option<AlbumActionButton>,
    /// content availability restrictions such as an age rating.
    pub content_restrictions: Option<ContentRestrictions>,
    /// milliseconds until an unreleased album becomes available, for pre-saves.
    pub milliseconds_until_release: Option<i64>,
    /// pre-save availability date, as iso 8601, for upcoming albums.
    pub presave_date: Option<String>,
}

impl Album {
    /// every track from every volume, flattened in playback order.
    #[must_use]
    pub fn tracks(&self) -> Vec<&Track> {
        self.volumes.iter().flatten().collect()
    }

    /// the album artists joined for display.
    #[must_use]
    pub fn artists_name(&self) -> String {
        self.artists
            .iter()
            .filter_map(|a| a.name.as_deref())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// the label names, whichever shape the endpoint returned them in.
    #[must_use]
    pub fn label_names(&self) -> Vec<&str> {
        self.labels.iter().filter_map(AlbumLabel::name).collect()
    }

    /// the cover url at a concrete size, for example `400x400`.
    #[must_use]
    pub fn cover_url(&self, size: &str) -> Option<String> {
        self.cover_uri
            .as_deref()
            .map(|uri| super::normalize_image_url(uri, size))
    }
}

/// an album together with its trailer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumTrailer {
    /// the album.
    pub album: Option<Album>,
    /// related artists.
    pub artists: Vec<Artist>,
    /// the trailer.
    pub trailer: Option<TrailerInfo>,
}

/// entities similar to an album.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumSimilarEntities {
    /// similar entity items, still free-form upstream.
    pub items: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_accept_both_object_and_bare_name_shapes() {
        let album: Album =
            serde_json::from_str(r#"{"labels":[{"id":1,"name":"Label One"},"Bare Name"]}"#)
                .unwrap();
        assert_eq!(album.label_names(), vec!["Label One", "Bare Name"]);
        assert!(matches!(album.labels[0], AlbumLabel::Full(_)));
        assert!(matches!(album.labels[1], AlbumLabel::Name(_)));
    }

    #[test]
    fn volumes_flatten_in_playback_order() {
        let album: Album =
            serde_json::from_str(r#"{"volumes":[[{"id":1},{"id":2}],[{"id":3}]]}"#).unwrap();
        let ids: Vec<String> = album
            .tracks()
            .iter()
            .map(|t| t.id.as_ref().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["1", "2", "3"]);
    }

    #[test]
    fn an_album_without_volumes_has_no_tracks() {
        assert!(Album::default().tracks().is_empty());
        assert!(Album::default().label_names().is_empty());
    }

    #[test]
    fn duplicates_nest_recursively() {
        let album: Album =
            serde_json::from_str(r#"{"id":1,"duplicates":[{"id":2,"title":"deluxe"}]}"#).unwrap();
        assert_eq!(album.duplicates[0].title.as_deref(), Some("deluxe"));
    }

    #[test]
    fn both_album_type_keys_are_kept() {
        // the api migrated from `type` to `albumType` but still sends both
        let album: Album =
            serde_json::from_str(r#"{"type":"single","albumType":"podcast"}"#).unwrap();
        assert_eq!(album.r#type.as_deref(), Some("single"));
        assert_eq!(album.album_type.as_deref(), Some("podcast"));
    }

    #[test]
    fn album_models_survive_an_empty_object() {
        serde_json::from_str::<Album>("{}").unwrap();
        serde_json::from_str::<Label>("{}").unwrap();
        serde_json::from_str::<AlbumTrailer>("{}").unwrap();
        serde_json::from_str::<AlbumSimilarEntities>("{}").unwrap();
        serde_json::from_str::<Deprecation>("{}").unwrap();
        serde_json::from_str::<TrackPosition>("{}").unwrap();
    }
}
