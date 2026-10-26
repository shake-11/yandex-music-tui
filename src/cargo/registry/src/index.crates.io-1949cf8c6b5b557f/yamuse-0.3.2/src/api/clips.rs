//! clip endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Clip, ClipsWillLike};
use crate::params::Params;

impl Client {
    /// fetch one or many clips by id.
    ///
    /// `GET /clips` — this one takes a comma-joined list, not a repeated field.
    pub async fn clips(
        &self,
        clip_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Vec<Clip>> {
        let joined = clip_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.get("/clips", Params::new().add("clipIds", joined))
            .await
    }

    /// fetch a recommended page of clips.
    ///
    /// `GET /clips/will/like`
    pub async fn clips_will_like(&self, page: u32, page_size: u32) -> Result<ClipsWillLike> {
        let params = Params::new().add("page", page).add("pageSize", page_size);
        self.get("/clips/will/like", params).await
    }
}
