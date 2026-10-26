//! album endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Album, AlbumSimilarEntities, AlbumTrailer};
use crate::params::{Params, segment};
use crate::types::BatchObject;

impl Client {
    /// fetch one or many albums by id, without their track lists.
    ///
    /// `POST /albums`
    pub async fn albums(
        &self,
        album_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Vec<Album>> {
        self.get_batch(BatchObject::Album, album_ids, Params::new())
            .await
    }

    /// fetch one album by id, or `None` when it does not resolve.
    pub async fn album(&self, album_id: impl std::fmt::Display) -> Result<Option<Album>> {
        Ok(self.albums([album_id]).await?.into_iter().next())
    }

    /// fetch an album together with all of its tracks, grouped by volume.
    ///
    /// `GET /albums/{id}/with-tracks`
    ///
    /// use [`Album::tracks`] to flatten the volumes into playback order.
    pub async fn album_with_tracks(&self, album_id: impl std::fmt::Display) -> Result<Album> {
        let album_id = segment(album_id);
        self.get(&format!("/albums/{album_id}/with-tracks"), Params::new())
            .await
    }

    /// fetch entities similar to an album.
    ///
    /// `GET /albums/{id}/similar-entities`
    pub async fn album_similar_entities(
        &self,
        album_id: impl std::fmt::Display,
    ) -> Result<AlbumSimilarEntities> {
        let album_id = segment(album_id);
        self.get(
            &format!("/albums/{album_id}/similar-entities"),
            Params::new(),
        )
        .await
    }

    /// fetch the trailer of an album.
    ///
    /// `GET /albums/{id}/trailer`
    pub async fn album_trailer(&self, album_id: impl std::fmt::Display) -> Result<AlbumTrailer> {
        let album_id = segment(album_id);
        self.get(&format!("/albums/{album_id}/trailer"), Params::new())
            .await
    }
}
