//! landing-page, chart, genre and feed endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{ChartInfo, Feed, Genre, Landing, LandingList, TagResult};
use crate::params::{Params, segment};

/// sample user id the landing endpoint requires in `eitherUserId`.
///
/// the endpoint refuses to answer without one; this is the value the official
/// clients send.
const LANDING_EITHER_USER_ID: &str = "10254713668400548221";

impl Client {
    /// fetch one or more landing blocks.
    ///
    /// `GET /landing3`
    ///
    /// block names include `personalplaylists`, `chart`, `new-releases`,
    /// `new-playlists` and `podcasts`.
    pub async fn landing(
        &self,
        blocks: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Landing> {
        let params = Params::new()
            .add_all("blocks", blocks)
            .add("eitherUserId", LANDING_EITHER_USER_ID);
        self.get("/landing3", params).await
    }

    /// fetch the global chart.
    ///
    /// `GET /landing3/chart`
    pub async fn chart(&self) -> Result<ChartInfo> {
        self.get("/landing3/chart", Params::new()).await
    }

    /// fetch a regional chart, for example `russia` or `world`.
    ///
    /// `GET /landing3/chart/{option}`
    pub async fn chart_for(&self, chart_option: &str) -> Result<ChartInfo> {
        let chart_option = segment(chart_option);
        self.get(&format!("/landing3/chart/{chart_option}"), Params::new())
            .await
    }

    /// fetch the "new releases" landing list.
    ///
    /// `GET /landing3/new-releases`
    pub async fn new_releases(&self) -> Result<LandingList> {
        self.get("/landing3/new-releases", Params::new()).await
    }

    /// fetch the "new playlists" landing list.
    ///
    /// `GET /landing3/new-playlists`
    pub async fn new_playlists(&self) -> Result<LandingList> {
        self.get("/landing3/new-playlists", Params::new()).await
    }

    /// fetch the podcasts landing list.
    ///
    /// `GET /landing3/podcasts`
    pub async fn podcasts(&self) -> Result<LandingList> {
        self.get("/landing3/podcasts", Params::new()).await
    }

    /// fetch the genre tree.
    ///
    /// `GET /genres`
    pub async fn genres(&self) -> Result<Vec<Genre>> {
        self.get("/genres", Params::new()).await
    }

    /// fetch the personalised feed.
    ///
    /// `GET /feed`
    pub async fn feed(&self) -> Result<Feed> {
        self.get("/feed", Params::new()).await
    }

    /// check whether the feed onboarding wizard has been completed.
    ///
    /// `GET /feed/wizard/is-passed`
    pub async fn feed_wizard_is_passed(&self) -> Result<bool> {
        let value = self
            .get_value("/feed/wizard/is-passed", Params::new())
            .await?;
        Ok(value
            .get("isWizardPassed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
    }

    /// fetch the playlists grouped under a tag.
    ///
    /// `GET /tags/{id}/playlist-ids`
    pub async fn tag_playlists(&self, tag_id: &str) -> Result<TagResult> {
        let tag_id = segment(tag_id);
        self.get(&format!("/tags/{tag_id}/playlist-ids"), Params::new())
            .await
    }
}
