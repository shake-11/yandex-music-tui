//! pinned-item endpoints.

use serde_json::json;

use crate::client::Client;
use crate::error::Result;
use crate::models::{Mutation, Pin, PinsList};
use crate::params::Params;

impl Client {
    /// fetch the user's pinned items.
    ///
    /// `GET /pins`
    pub async fn pins(&self) -> Result<PinsList> {
        self.get("/pins", Params::new()).await
    }

    /// pin an album.
    ///
    /// `PUT /pin/album`
    pub async fn pin_album(&self, album_id: i64) -> Result<Pin> {
        let value = self
            .put_value("/pin/album", &json!({ "id": album_id }))
            .await?;
        self.decode(value)
    }

    /// unpin an album.
    ///
    /// `DELETE /pin/album`
    pub async fn unpin_album(&self, album_id: i64) -> Result<Mutation> {
        let value = self
            .delete_value("/pin/album", &json!({ "id": album_id }))
            .await?;
        Client::acknowledged(&value)
    }

    /// pin an artist.
    ///
    /// `PUT /pin/artist`
    pub async fn pin_artist(&self, artist_id: i64) -> Result<Pin> {
        let value = self
            .put_value("/pin/artist", &json!({ "id": artist_id }))
            .await?;
        self.decode(value)
    }

    /// unpin an artist.
    ///
    /// `DELETE /pin/artist`
    pub async fn unpin_artist(&self, artist_id: i64) -> Result<Mutation> {
        let value = self
            .delete_value("/pin/artist", &json!({ "id": artist_id }))
            .await?;
        Client::acknowledged(&value)
    }

    /// pin a playlist.
    ///
    /// `PUT /pin/playlist`
    pub async fn pin_playlist(&self, uid: i64, kind: i64) -> Result<Pin> {
        let value = self
            .put_value("/pin/playlist", &json!({ "uid": uid, "kind": kind }))
            .await?;
        self.decode(value)
    }

    /// unpin a playlist.
    ///
    /// `DELETE /pin/playlist`
    pub async fn unpin_playlist(&self, uid: i64, kind: i64) -> Result<Mutation> {
        let value = self
            .delete_value("/pin/playlist", &json!({ "uid": uid, "kind": kind }))
            .await?;
        Client::acknowledged(&value)
    }

    /// pin a wave, identified by its radio seed — for example `artist:12345`.
    ///
    /// `PUT /pin/wave`
    pub async fn pin_wave(&self, seeds: &str) -> Result<Pin> {
        let value = self
            .put_value("/pin/wave", &json!({ "seeds": seeds }))
            .await?;
        self.decode(value)
    }

    /// unpin a wave.
    ///
    /// `DELETE /pin/wave`
    pub async fn unpin_wave(&self, seeds: &str) -> Result<Mutation> {
        let value = self
            .delete_value("/pin/wave", &json!({ "seeds": seeds }))
            .await?;
        Client::acknowledged(&value)
    }
}
