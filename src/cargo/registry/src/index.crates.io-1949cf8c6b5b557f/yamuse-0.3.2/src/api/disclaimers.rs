//! legal-disclaimer endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::Disclaimer;
use crate::params::{Params, segment};

impl Client {
    /// fetch the disclaimer of a track.
    ///
    /// `GET /tracks/{id}/disclaimer`
    pub async fn track_disclaimer(&self, track_id: impl std::fmt::Display) -> Result<Disclaimer> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/disclaimer"), Params::new())
            .await
    }

    /// fetch the disclaimer of a clip.
    ///
    /// `GET /clips/{id}/disclaimer`
    pub async fn clip_disclaimer(&self, clip_id: impl std::fmt::Display) -> Result<Disclaimer> {
        let clip_id = segment(clip_id);
        self.get(&format!("/clips/{clip_id}/disclaimer"), Params::new())
            .await
    }

    /// fetch the disclaimer of an album.
    ///
    /// `GET /albums/{id}/disclaimer`
    pub async fn album_disclaimer(&self, album_id: impl std::fmt::Display) -> Result<Disclaimer> {
        let album_id = segment(album_id);
        self.get(&format!("/albums/{album_id}/disclaimer"), Params::new())
            .await
    }

    /// fetch the disclaimer of an artist.
    ///
    /// `GET /artists/{id}/disclaimer`
    pub async fn artist_disclaimer(&self, artist_id: impl std::fmt::Display) -> Result<Disclaimer> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/disclaimer"), Params::new())
            .await
    }
}
