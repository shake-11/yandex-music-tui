//! liked and disliked library entries.

use serde::{Deserialize, Serialize};

use super::Id;
use super::album::Album;
use super::artist::Artist;
use super::playlist::Playlist;
use super::track::TrackShort;

/// a liked entity — artist, album or playlist — with when it was liked.
///
/// which of the three payload fields is populated follows [`r#type`](Self::type).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Like {
    /// liked entity type.
    pub r#type: Option<String>,
    /// entity id.
    ///
    /// the live api sends a number for an album like and a `"uid:kind"` string
    /// for a playlist one, so it is an [`Id`] rather than either — call
    /// [`Id::as_number`] for the numeric form.
    pub id: Option<Id>,
    /// when the like was added.
    pub timestamp: Option<String>,
    /// liked album, when `type` is `album`.
    pub album: Option<Album>,
    /// liked artist, when `type` is `artist`.
    pub artist: Option<Artist>,
    /// liked playlist, when `type` is `playlist`.
    pub playlist: Option<Playlist>,
    /// short description.
    pub short_description: Option<String>,
    /// description.
    pub description: Option<String>,
    /// whether a premiere.
    pub is_premiere: Option<bool>,
    /// whether shown as a banner.
    pub is_banner: Option<bool>,
}

/// a revisioned list of track references — the user's liked or disliked library.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TracksList {
    /// owner uid.
    pub uid: Option<i64>,
    /// library revision.
    pub revision: Option<i64>,
    /// stable uuid of the backing "liked tracks" playlist.
    pub playlist_uuid: Option<String>,
    /// the track references.
    pub tracks: Vec<TrackShort>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_like_carries_only_the_payload_matching_its_type() {
        let album_like: Like =
            serde_json::from_str(r#"{"type":"album","album":{"id":1,"title":"A"}}"#).unwrap();
        assert_eq!(album_like.album.unwrap().title.as_deref(), Some("A"));
        assert!(album_like.artist.is_none());
        assert!(album_like.playlist.is_none());
    }

    #[test]
    fn an_album_like_id_arrives_as_a_number_and_a_playlist_one_as_a_string() {
        // the live api sends `"id": 38695104` for an album like. a whole library
        // of them is one mistyped field per element, which blew through
        // `tolerant::MAX_REPAIRS` and failed the call outright.
        let album: Like = serde_json::from_str(r#"{"type":"album","id":38695104}"#).unwrap();
        assert_eq!(album.id.as_ref().unwrap().as_number(), Some(38_695_104));
        assert_eq!(album.id.unwrap().to_string(), "38695104");

        let playlist: Like = serde_json::from_str(r#"{"type":"playlist","id":"123:456"}"#).unwrap();
        assert_eq!(playlist.id.unwrap().to_string(), "123:456");
    }

    #[test]
    fn a_full_page_of_album_likes_parses_without_spending_a_single_repair() {
        // the regression this guards: the budget is per response, so a field
        // mistyped once in the model costs one repair per element.
        let raw = format!(
            "[{}]",
            (0..64)
                .map(|i| format!(r#"{{"type":"album","id":{i},"album":{{"id":{i}}}}}"#))
                .collect::<Vec<_>>()
                .join(",")
        );

        let (likes, repairs): (Vec<Like>, _) =
            crate::tolerant::deserialize(serde_json::from_str(&raw).unwrap()).unwrap();

        assert_eq!(likes.len(), 64);
        assert!(repairs.is_empty(), "{repairs:?}");
    }

    #[test]
    fn a_tracks_list_carries_its_revision() {
        let list: TracksList =
            serde_json::from_str(r#"{"uid":1,"revision":42,"tracks":[{"id":"10:20"}]}"#).unwrap();
        assert_eq!(list.revision, Some(42));
        assert_eq!(list.tracks[0].id.as_ref().unwrap().to_string(), "10:20");
    }

    #[test]
    fn like_models_survive_an_empty_object() {
        serde_json::from_str::<Like>("{}").unwrap();
        serde_json::from_str::<TracksList>("{}").unwrap();
    }
}
