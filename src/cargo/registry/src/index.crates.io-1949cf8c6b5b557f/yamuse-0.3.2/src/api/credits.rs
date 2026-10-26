//! production-credit endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::Credits;
use crate::params::{Params, segment};

impl Client {
    /// fetch the production credits of a track.
    ///
    /// `GET /tracks/{id}/credits`
    pub async fn track_credits(&self, track_id: impl std::fmt::Display) -> Result<Credits> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/credits"), Params::new())
            .await
    }

    /// fetch the production credits of a clip.
    ///
    /// `GET /clips/{id}/credits`
    pub async fn clip_credits(&self, clip_id: impl std::fmt::Display) -> Result<Credits> {
        let clip_id = segment(clip_id);
        self.get(&format!("/clips/{clip_id}/credits"), Params::new())
            .await
    }
}
