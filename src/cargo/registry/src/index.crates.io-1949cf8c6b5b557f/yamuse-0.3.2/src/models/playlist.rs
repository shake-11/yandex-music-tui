//! the playlist model, its promo decorations and its generated variants.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::artist::Artist;
use super::misc::TrailerInfo;
use super::track::{Track, TrackId, TrackShort};
use super::user::User;
use super::{Cover, CoverDerivedColors, Pager};

/// grammatical case forms of a name, used to inflect "made for {user}" labels.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CaseForms {
    /// nominative case.
    pub nominative: Option<String>,
    /// genitive case.
    pub genitive: Option<String>,
    /// dative case.
    pub dative: Option<String>,
    /// accusative case.
    pub accusative: Option<String>,
    /// instrumental case.
    pub instrumental: Option<String>,
    /// prepositional case.
    pub prepositional: Option<String>,
}

/// personalization descriptor for a "made for you" playlist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MadeFor {
    /// the user the playlist was made for.
    pub user_info: Option<User>,
    /// inflected forms of the user's name.
    pub case_forms: Option<CaseForms>,
}

/// aggregate play counter shown on some personalized playlists.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlayCounter {
    /// number of plays.
    pub value: Option<i64>,
    /// human-readable description.
    pub description: Option<String>,
    /// whether the counter was updated in this period.
    pub updated: Option<bool>,
}

/// explains why a previously available playlist is now absent.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistAbsence {
    /// kind of the absent playlist.
    pub kind: Option<i64>,
    /// reason the playlist is unavailable.
    pub reason: Option<String>,
}

/// open graph metadata used when a playlist is shared as a link.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OpenGraphData {
    /// share title.
    pub title: Option<String>,
    /// share description.
    pub description: Option<String>,
    /// share image.
    pub image: Option<Cover>,
}

/// branding applied to a sponsored or branded playlist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Brand {
    /// brand image.
    pub image: Option<String>,
    /// background image.
    pub background: Option<String>,
    /// reference (target) url.
    pub reference: Option<String>,
    /// tracking pixel urls.
    pub pixels: Vec<String>,
    /// theme name.
    pub theme: Option<String>,
    /// playlist-specific theme name.
    pub playlist_theme: Option<String>,
    /// button label or color.
    pub button: Option<String>,
}

/// availability of a playlist trailer.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistAvailability {
    /// whether the trailer is available.
    pub available: Option<bool>,
}

/// custom "wave" (radio) presentation attached to an entity.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomWave {
    /// wave title.
    pub title: Option<String>,
    /// animation url.
    pub animation_url: Option<String>,
    /// title position.
    pub position: Option<String>,
    /// header text.
    pub header: Option<String>,
    /// background image url.
    pub background_image_url: Option<String>,
    /// square-format agent animation url.
    pub square_agent_animation: Option<String>,
}

/// contest entry state for a playlist submitted to a contest.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Contest {
    /// contest id.
    pub contest_id: Option<String>,
    /// submission status.
    pub status: Option<String>,
    /// whether the playlist can still be edited.
    pub can_edit: Option<bool>,
    /// submission timestamp.
    pub sent: Option<String>,
    /// withdrawal timestamp.
    pub withdrawn: Option<String>,
}

/// a lightweight reference to a playlist by owner uid and kind.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistId {
    /// owner user id.
    pub uid: Option<i64>,
    /// playlist kind, the per-owner ordinal.
    pub kind: Option<i64>,
}

impl PlaylistId {
    /// the canonical `uid:kind` identifier, when both parts are present.
    #[must_use]
    pub fn as_string(&self) -> Option<String> {
        Some(format!("{}:{}", self.uid?, self.kind?))
    }
}

/// a playlist, user-made or editorial.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Playlist {
    /// owner of the playlist.
    pub owner: Option<User>,
    /// playlist cover.
    pub cover: Option<Cover>,
    /// "made for" personalization.
    pub made_for: Option<MadeFor>,
    /// play counter.
    pub play_counter: Option<PlayCounter>,
    /// absence info.
    pub playlist_absence: Option<PlaylistAbsence>,
    /// owner uid.
    pub uid: Option<i64>,
    /// playlist kind, the id within the owner's playlists.
    pub kind: Option<i64>,
    /// title.
    pub title: Option<String>,
    /// number of tracks.
    pub track_count: Option<i64>,
    /// tags, still free-form upstream.
    pub tags: Vec<Value>,
    /// revision number.
    pub revision: Option<i64>,
    /// snapshot number.
    pub snapshot: Option<i64>,
    /// visibility: `public` or `private`.
    pub visibility: Option<String>,
    /// whether collaborative.
    pub collective: Option<bool>,
    /// url slug.
    pub url_part: Option<String>,
    /// creation timestamp.
    pub created: Option<String>,
    /// last modification timestamp.
    pub modified: Option<String>,
    /// whether available.
    pub available: Option<bool>,
    /// whether shown as a banner.
    pub is_banner: Option<bool>,
    /// whether a premiere.
    pub is_premiere: Option<bool>,
    /// total duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// open graph image uri template.
    pub og_image: Option<String>,
    /// open graph title.
    pub og_title: Option<String>,
    /// open graph description.
    pub og_description: Option<String>,
    /// image uri template.
    pub image: Option<String>,
    /// cover without overlaid text.
    pub cover_without_text: Option<Cover>,
    /// contest info.
    pub contest: Option<Contest>,
    /// background color.
    pub background_color: Option<String>,
    /// text color.
    pub text_color: Option<String>,
    /// origin id used in `from` parameters.
    pub id_for_from: Option<String>,
    /// open graph data.
    pub og_data: Option<OpenGraphData>,
    /// branding.
    pub branding: Option<Brand>,
    /// metrika id.
    pub metrika_id: Option<i64>,
    /// co-author uids.
    pub coauthors: Vec<i64>,
    /// top artists in the playlist.
    pub top_artist: Vec<Artist>,
    /// recently added track references.
    pub recent_tracks: Vec<TrackId>,
    /// track references.
    pub tracks: Vec<TrackShort>,
    /// number of likes.
    pub likes_count: Option<i64>,
    /// playlists similar to this one.
    pub similar_playlists: Vec<Playlist>,
    /// other playlists by the same owner.
    pub last_owner_playlists: Vec<Playlist>,
    /// generated playlist kind, for example `playlistOfTheDay`.
    pub generated_playlist_type: Option<String>,
    /// animated cover uri.
    pub animated_cover_uri: Option<String>,
    /// whether the playlist was ever played.
    pub ever_played: Option<bool>,
    /// description.
    pub description: Option<String>,
    /// formatted (html) description.
    pub description_formatted: Option<String>,
    /// stable uuid.
    pub playlist_uuid: Option<String>,
    /// playlist kind label.
    pub r#type: Option<String>,
    /// whether generation finished.
    pub ready: Option<bool>,
    /// custom wave config.
    pub custom_wave: Option<CustomWave>,
    /// pagination metadata for the track list.
    pub pager: Option<Pager>,
    /// whether a trailer exists.
    pub has_trailer: Option<bool>,
    /// trailer availability.
    pub trailer: Option<PlaylistAvailability>,
    /// background video url.
    pub background_video_url: Option<String>,
    /// background video id.
    pub background_video_id: Option<String>,
    /// background image url.
    pub background_image_url: Option<String>,
    /// colors derived from the cover image.
    pub derived_colors: Option<CoverDerivedColors>,
    /// regions where the playlist is available, still free-form upstream.
    pub regions: Vec<Value>,
    /// artist-playlist kind, when this is an artist's playlist.
    pub artist_playlist_type: Option<String>,
    /// call-to-action button, still free-form upstream.
    pub action_button: Option<Value>,
}

impl Playlist {
    /// the canonical `uid:kind` identifier, when both parts are present.
    #[must_use]
    pub fn playlist_id(&self) -> Option<String> {
        Some(format!("{}:{}", self.uid?, self.kind?))
    }

    /// the full tracks the endpoint inlined, skipping bare references.
    #[must_use]
    pub fn full_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter_map(|short| short.track.as_ref())
            .collect()
    }
}

/// track recommendations generated for a playlist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistRecommendations {
    /// the recommended tracks.
    pub tracks: Vec<Track>,
    /// identifier of the recommendation batch.
    pub batch_id: Option<String>,
}

/// entities similar to a playlist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistSimilarEntities {
    /// similar entity items, still free-form upstream.
    pub items: Vec<Value>,
}

/// a list of playlists returned by the bulk endpoint.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistsList {
    /// the playlists.
    pub playlists: Vec<Playlist>,
}

/// a playlist together with its trailer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistTrailer {
    /// the playlist.
    pub playlist: Option<Playlist>,
    /// the trailer.
    pub trailer: Option<TrailerInfo>,
    /// whether the trailer is shareable.
    pub shareable: Option<bool>,
}

/// an automatically generated personal playlist, such as "playlist of the day".
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneratedPlaylist {
    /// the generator type: `playlistOfTheDay`, `origin`, `recentTracks`,
    /// `neverHeard`, `podcasts`, `missedLikes`.
    pub r#type: Option<String>,
    /// whether the playlist has been generated and is ready.
    pub ready: Option<bool>,
    /// whether the user should be notified about the playlist.
    pub notify: Option<bool>,
    /// the underlying playlist.
    pub data: Option<Playlist>,
    /// rich description blocks, still free-form upstream.
    pub description: Vec<Value>,
    /// plain-text preview description.
    pub preview_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_ids_compose_from_uid_and_kind() {
        let playlist: Playlist = serde_json::from_str(r#"{"uid":123,"kind":1000}"#).unwrap();
        assert_eq!(playlist.playlist_id().as_deref(), Some("123:1000"));
        assert!(Playlist::default().playlist_id().is_none());

        let reference = PlaylistId {
            uid: Some(9),
            kind: Some(3),
        };
        assert_eq!(reference.as_string().as_deref(), Some("9:3"));
        assert!(PlaylistId::default().as_string().is_none());
    }

    #[test]
    fn full_tracks_skip_bare_references() {
        let playlist: Playlist = serde_json::from_str(
            r#"{"tracks":[{"id":1,"track":{"id":1,"title":"inlined"}},{"id":2}]}"#,
        )
        .unwrap();
        assert_eq!(playlist.tracks.len(), 2);
        let full = playlist.full_tracks();
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].title.as_deref(), Some("inlined"));
    }

    #[test]
    fn similar_playlists_nest_recursively() {
        let playlist: Playlist =
            serde_json::from_str(r#"{"kind":1,"similarPlaylists":[{"kind":2}]}"#).unwrap();
        assert_eq!(playlist.similar_playlists[0].kind, Some(2));
    }

    #[test]
    fn playlist_models_survive_an_empty_object() {
        serde_json::from_str::<Playlist>("{}").unwrap();
        serde_json::from_str::<GeneratedPlaylist>("{}").unwrap();
        serde_json::from_str::<PlaylistsList>("{}").unwrap();
        serde_json::from_str::<PlaylistTrailer>("{}").unwrap();
        serde_json::from_str::<PlaylistRecommendations>("{}").unwrap();
        serde_json::from_str::<PlaylistSimilarEntities>("{}").unwrap();
        serde_json::from_str::<MadeFor>("{}").unwrap();
        serde_json::from_str::<Brand>("{}").unwrap();
        serde_json::from_str::<Contest>("{}").unwrap();
        serde_json::from_str::<CustomWave>("{}").unwrap();
    }
}
