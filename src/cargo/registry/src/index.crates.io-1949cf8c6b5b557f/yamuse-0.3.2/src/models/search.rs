//! search results and suggestions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::album::Album;
use super::artist::Artist;
use super::clip::Clip;
use super::playlist::Playlist;
use super::track::Track;
use super::user::User;
use super::video::Video;

/// one typed block of search results, for example every matching track.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchResult<T> {
    /// result type: `track`, `artist`, `album`, `playlist`, `video`, `user`.
    pub r#type: Option<String>,
    /// total number of matches.
    pub total: Option<u32>,
    /// maximum results per page.
    pub per_page: Option<u32>,
    /// display order of this block.
    pub order: Option<u32>,
    /// the matched entities.
    pub results: Vec<T>,
}

// derived `Default` would demand `T: Default`; results is a Vec, so it does not.
impl<T> Default for SearchResult<T> {
    fn default() -> Self {
        Self {
            r#type: None,
            total: None,
            per_page: None,
            order: None,
            results: Vec::new(),
        }
    }
}

impl<T> SearchResult<T> {
    /// whether another page of this block exists beyond the one requested.
    #[must_use]
    pub fn has_more(&self, page: u32) -> bool {
        match (self.total, self.per_page) {
            (Some(total), Some(per_page)) if per_page > 0 => (page + 1) * per_page < total,
            _ => false,
        }
    }
}

/// the resolved entity behind a [`Best`] match.
///
/// the api tags the best match with a `type` and puts the matching entity in
/// `result`; [`Best::entity`] resolves the pair, preserving an unrecognized type
/// as raw json rather than dropping it.
///
/// the catalogue variants are boxed: an inline `Album` alone runs to about
/// 1.7 kb, which every `entity()` call would then move by value.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SearchEntity {
    /// a track.
    Track(Box<Track>),
    /// an artist.
    Artist(Box<Artist>),
    /// an album, which also covers podcasts.
    Album(Box<Album>),
    /// a playlist.
    Playlist(Box<Playlist>),
    /// a music video.
    Video(Box<Video>),
    /// a user.
    User(Box<User>),
    /// a clip.
    Clip(Box<Clip>),
    /// a type this crate does not model yet; the payload is preserved verbatim.
    Unknown(Value),
}

/// the single best match for a query, with its resolved entity.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Best {
    /// entity type of the best match.
    pub r#type: Option<String>,
    /// the raw entity payload; call [`entity`](Self::entity) for the typed form.
    pub result: Option<Value>,
    /// echoed query text.
    pub text: Option<String>,
}

impl Best {
    /// resolve the payload into its typed form according to `type`.
    ///
    /// returns `None` only when there is no payload. an unrecognized or
    /// unexpectedly-shaped payload becomes [`SearchEntity::Unknown`] — a search
    /// must never fail over one unfamiliar best match.
    #[must_use]
    pub fn entity(&self) -> Option<SearchEntity> {
        let raw = self.result.clone()?;
        let typed = |value: Value| -> Option<SearchEntity> {
            Some(match self.r#type.as_deref()? {
                "track" => SearchEntity::Track(Box::new(serde_json::from_value(value).ok()?)),
                "artist" => SearchEntity::Artist(Box::new(serde_json::from_value(value).ok()?)),
                "album" | "podcast" => {
                    SearchEntity::Album(Box::new(serde_json::from_value(value).ok()?))
                },
                "playlist" => SearchEntity::Playlist(Box::new(serde_json::from_value(value).ok()?)),
                "video" => SearchEntity::Video(Box::new(serde_json::from_value(value).ok()?)),
                "user" => SearchEntity::User(Box::new(serde_json::from_value(value).ok()?)),
                "clip" => SearchEntity::Clip(Box::new(serde_json::from_value(value).ok()?)),
                _ => return None,
            })
        };
        Some(typed(raw.clone()).unwrap_or(SearchEntity::Unknown(raw)))
    }
}

/// search suggestions for a partial query.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Suggestions {
    /// the best matching entity for the partial query.
    pub best: Option<Best>,
    /// suggested completion strings.
    pub suggestions: Vec<String>,
}

/// the full search response across every entity type.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Search {
    /// unique id of the search request.
    pub search_request_id: Option<String>,
    /// echoed query text.
    pub text: Option<String>,
    /// the single best match.
    pub best: Option<Best>,
    /// matching albums.
    pub albums: Option<SearchResult<Album>>,
    /// matching artists.
    pub artists: Option<SearchResult<Artist>>,
    /// matching playlists.
    pub playlists: Option<SearchResult<Playlist>>,
    /// matching tracks.
    pub tracks: Option<SearchResult<Track>>,
    /// matching videos.
    pub videos: Option<SearchResult<Video>>,
    /// matching users.
    pub users: Option<SearchResult<User>>,
    /// matching podcasts, modeled as albums.
    pub podcasts: Option<SearchResult<Album>>,
    /// matching podcast episodes, modeled as tracks.
    pub podcast_episodes: Option<SearchResult<Track>>,
    /// matching clips.
    pub clips: Option<SearchResult<Clip>>,
    /// result type filter that was applied.
    pub r#type: Option<String>,
    /// page index.
    pub page: Option<u32>,
    /// results per page.
    pub per_page: Option<u32>,
    /// corrected query text, when a misspelling was detected.
    pub misspell_result: Option<String>,
    /// original, mis-spelled query text.
    pub misspell_original: Option<String>,
    /// whether the query was auto-corrected.
    pub misspell_corrected: Option<bool>,
    /// whether correction was disabled.
    pub nocorrect: Option<bool>,
}

impl Search {
    /// every matched track, or an empty slice when the block is absent.
    #[must_use]
    pub fn track_results(&self) -> &[Track] {
        self.tracks
            .as_ref()
            .map_or(&[], |block| block.results.as_slice())
    }

    /// the corrected query when the api fixed a typo, otherwise `None`.
    #[must_use]
    pub fn correction(&self) -> Option<&str> {
        if self.misspell_corrected == Some(true) {
            self.misspell_result.as_deref()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best(json: &str) -> Best {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn best_matches_resolve_by_their_type_tag() {
        assert!(matches!(
            best(r#"{"type":"track","result":{"id":1,"title":"T"}}"#).entity(),
            Some(SearchEntity::Track(t)) if t.title.as_deref() == Some("T")
        ));
        assert!(matches!(
            best(r#"{"type":"artist","result":{"id":2}}"#).entity(),
            Some(SearchEntity::Artist(_))
        ));
        assert!(matches!(
            best(r#"{"type":"album","result":{"id":3}}"#).entity(),
            Some(SearchEntity::Album(_))
        ));
        assert!(matches!(
            best(r#"{"type":"playlist","result":{"kind":4}}"#).entity(),
            Some(SearchEntity::Playlist(_))
        ));
        assert!(matches!(
            best(r#"{"type":"video","result":{"title":"V"}}"#).entity(),
            Some(SearchEntity::Video(_))
        ));
        assert!(matches!(
            best(r#"{"type":"user","result":{"uid":5}}"#).entity(),
            Some(SearchEntity::User(_))
        ));
        assert!(matches!(
            best(r#"{"type":"clip","result":{"clipId":6}}"#).entity(),
            Some(SearchEntity::Clip(_))
        ));
    }

    #[test]
    fn podcasts_resolve_as_albums_because_that_is_how_the_api_models_them() {
        assert!(matches!(
            best(r#"{"type":"podcast","result":{"id":7,"metaType":"podcast"}}"#).entity(),
            Some(SearchEntity::Album(_))
        ));
    }

    #[test]
    fn an_unknown_best_type_keeps_its_payload() {
        match best(r#"{"type":"something-new","result":{"x":1}}"#).entity() {
            Some(SearchEntity::Unknown(value)) => assert_eq!(value, serde_json::json!({"x": 1})),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn a_best_without_a_payload_resolves_to_nothing() {
        assert!(best(r#"{"type":"track"}"#).entity().is_none());
        assert!(best("{}").entity().is_none());
    }

    #[test]
    fn a_full_search_response_deserializes() {
        let search: Search = serde_json::from_str(
            r#"{
                "searchRequestId":"r1","text":"query","page":0,"perPage":20,
                "best":{"type":"track","result":{"id":1}},
                "tracks":{"type":"track","total":100,"perPage":20,"results":[{"id":1},{"id":2}]},
                "albums":{"total":5,"results":[{"id":10}]}
            }"#,
        )
        .unwrap();
        assert_eq!(search.track_results().len(), 2);
        assert_eq!(search.albums.as_ref().unwrap().results.len(), 1);
        assert!(search.tracks.as_ref().unwrap().has_more(0));
        assert!(!search.tracks.as_ref().unwrap().has_more(4));
    }

    #[test]
    fn corrections_are_reported_only_when_actually_applied() {
        let corrected: Search = serde_json::from_str(
            r#"{"misspellCorrected":true,"misspellResult":"fixed","misspellOriginal":"fxied"}"#,
        )
        .unwrap();
        assert_eq!(corrected.correction(), Some("fixed"));

        let not_corrected: Search = serde_json::from_str(r#"{"misspellResult":"fixed"}"#).unwrap();
        assert_eq!(not_corrected.correction(), None);
    }

    #[test]
    fn an_absent_track_block_yields_an_empty_slice_not_a_panic() {
        let search: Search = serde_json::from_str("{}").unwrap();
        assert!(search.track_results().is_empty());
    }

    #[test]
    fn search_models_survive_an_empty_object() {
        serde_json::from_str::<Search>("{}").unwrap();
        serde_json::from_str::<Best>("{}").unwrap();
        serde_json::from_str::<Suggestions>("{}").unwrap();
        serde_json::from_str::<SearchResult<Track>>("{}").unwrap();
    }
}
