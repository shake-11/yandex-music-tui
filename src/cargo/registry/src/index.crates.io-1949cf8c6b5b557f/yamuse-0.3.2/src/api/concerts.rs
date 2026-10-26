//! concert endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{
    ArtistConcerts, ConcertFeed, ConcertInfo, ConcertLocations, ConcertSkeleton, ConcertTabConfig,
};
use crate::params::{Params, segment};

impl Client {
    /// fetch an artist's upcoming concerts.
    ///
    /// `GET /artists/{id}/concerts`
    pub async fn artist_concerts(
        &self,
        artist_id: impl std::fmt::Display,
    ) -> Result<ArtistConcerts> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/concerts"), Params::new())
            .await
    }

    /// fetch detailed information about a concert.
    ///
    /// `GET /concerts/{id}/info`
    pub async fn concert_info(&self, concert_id: impl std::fmt::Display) -> Result<ConcertInfo> {
        let concert_id = segment(concert_id);
        self.get(&format!("/concerts/{concert_id}/info"), Params::new())
            .await
    }

    /// fetch the render scaffold of a concert page.
    ///
    /// `GET /concerts/{id}/skeletons/{skeleton_id}`
    pub async fn concert_skeleton(
        &self,
        concert_id: impl std::fmt::Display,
        skeleton_id: &str,
    ) -> Result<ConcertSkeleton> {
        let concert_id = segment(concert_id);
        let skeleton_id = segment(skeleton_id);
        self.get(
            &format!("/concerts/{concert_id}/skeletons/{skeleton_id}"),
            Params::new(),
        )
        .await
    }

    /// fetch the concert feed, optionally filtered by location.
    ///
    /// `GET /concerts/feed`
    pub async fn concerts_feed(
        &self,
        locations: impl IntoIterator<Item = i64>,
    ) -> Result<ConcertFeed> {
        let joined = locations
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let params = if joined.is_empty() {
            Params::new()
        } else {
            Params::new().add("locations", joined)
        };
        self.get("/concerts/feed", params).await
    }

    /// fetch the available concert locations.
    ///
    /// `GET /concerts/locations`
    pub async fn concerts_locations(&self) -> Result<ConcertLocations> {
        self.get("/concerts/locations", Params::new()).await
    }

    /// fetch the concert tab configuration.
    ///
    /// `GET /concerts/tab-config`
    pub async fn concerts_tab_config(&self) -> Result<ConcertTabConfig> {
        self.get("/concerts/tab-config", Params::new()).await
    }
}
