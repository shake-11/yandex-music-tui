//! listening-history endpoints.

use serde_json::{Value, json};

use crate::client::Client;
use crate::error::Result;
use crate::models::{MusicHistory, MusicHistoryItems};
use crate::params::Params;

/// the set of entities whose history items to resolve.
///
/// build it up with the `with_*` methods, then pass it to
/// [`Client::music_history_items`].
#[derive(Debug, Default, Clone)]
pub struct HistoryQuery {
    tracks: Vec<(i64, i64)>,
    albums: Vec<i64>,
    artists: Vec<i64>,
    playlists: Vec<(i64, i64)>,
    waves: Vec<Vec<String>>,
}

impl HistoryQuery {
    /// start an empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// add a track, identified by its own id and its album's.
    #[must_use]
    pub fn with_track(mut self, track_id: i64, album_id: i64) -> Self {
        self.tracks.push((track_id, album_id));
        self
    }

    /// add an album.
    #[must_use]
    pub fn with_album(mut self, album_id: i64) -> Self {
        self.albums.push(album_id);
        self
    }

    /// add an artist.
    #[must_use]
    pub fn with_artist(mut self, artist_id: i64) -> Self {
        self.artists.push(artist_id);
        self
    }

    /// add a playlist, identified by owner uid and kind.
    #[must_use]
    pub fn with_playlist(mut self, uid: i64, kind: i64) -> Self {
        self.playlists.push((uid, kind));
        self
    }

    /// add a wave, identified by its seeds — for example `["user:onyourwave"]`.
    #[must_use]
    pub fn with_wave(mut self, seeds: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.waves.push(seeds.into_iter().map(Into::into).collect());
        self
    }

    /// whether nothing was added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
            && self.waves.is_empty()
    }

    /// the request body: a typed item per entity, as the endpoint expects.
    fn to_items(&self) -> Vec<Value> {
        let mut items = Vec::new();
        for (track_id, album_id) in &self.tracks {
            items.push(json!({
                "type": "track",
                "data": { "itemId": { "trackId": track_id.to_string(), "albumId": album_id.to_string() } }
            }));
        }
        for album_id in &self.albums {
            items.push(
                json!({ "type": "album", "data": { "itemId": { "id": album_id.to_string() } } }),
            );
        }
        for artist_id in &self.artists {
            items.push(
                json!({ "type": "artist", "data": { "itemId": { "id": artist_id.to_string() } } }),
            );
        }
        for (uid, kind) in &self.playlists {
            items.push(
                json!({ "type": "playlist", "data": { "itemId": { "uid": uid, "kind": kind } } }),
            );
        }
        for seeds in &self.waves {
            items.push(json!({ "type": "wave", "data": { "itemId": { "seeds": seeds } } }));
        }
        items
    }
}

impl Client {
    /// fetch the user's listening history, grouped by day.
    ///
    /// `GET /music-history`
    ///
    /// `full_models_count` asks the api to inline that many full entities rather
    /// than bare ids; `0` keeps the response small.
    pub async fn music_history(&self, full_models_count: u32) -> Result<MusicHistory> {
        let params = Params::new().add("fullModelsCount", full_models_count);
        self.get("/music-history", params).await
    }

    /// resolve history items for a specific set of entities.
    ///
    /// `POST /music-history/items`
    pub async fn music_history_items(&self, query: &HistoryQuery) -> Result<MusicHistoryItems> {
        let body = json!({ "items": query.to_items() });
        let value = self.post_json_value("/music-history/items", &body).await?;
        self.decode(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_builds_no_items() {
        let query = HistoryQuery::new();
        assert!(query.is_empty());
        assert!(query.to_items().is_empty());
    }

    #[test]
    fn track_items_stringify_both_ids_as_the_endpoint_expects() {
        let items = HistoryQuery::new().with_track(42, 1001).to_items();
        assert_eq!(
            items[0],
            json!({"type":"track","data":{"itemId":{"trackId":"42","albumId":"1001"}}})
        );
    }

    #[test]
    fn playlist_items_keep_their_ids_numeric() {
        // unlike tracks, playlists are sent as numbers here
        let items = HistoryQuery::new().with_playlist(7, 1000).to_items();
        assert_eq!(
            items[0],
            json!({"type":"playlist","data":{"itemId":{"uid":7,"kind":1000}}})
        );
    }

    #[test]
    fn every_entity_kind_produces_one_item_in_a_stable_order() {
        let items = HistoryQuery::new()
            .with_track(1, 2)
            .with_album(3)
            .with_artist(4)
            .with_playlist(5, 6)
            .with_wave(["user:onyourwave"])
            .to_items();

        let types: Vec<&str> = items.iter().map(|i| i["type"].as_str().unwrap()).collect();
        assert_eq!(types, vec!["track", "album", "artist", "playlist", "wave"]);
        assert_eq!(
            items[4]["data"]["itemId"]["seeds"],
            json!(["user:onyourwave"])
        );
    }

    #[test]
    fn repeated_entities_of_one_kind_all_appear() {
        let items = HistoryQuery::new()
            .with_album(1)
            .with_album(2)
            .with_album(3)
            .to_items();
        assert_eq!(items.len(), 3);
    }
}
