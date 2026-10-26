//! metatag endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Metatag, MetatagAlbums, MetatagArtists, MetatagPlaylists, Metatags};
use crate::params::{Params, segment};

/// per-section item counts requested from [`Client::metatag`].
#[derive(Debug, Default, Clone)]
pub struct MetatagQuery {
    /// how many tracks to include.
    pub tracks_count: Option<u32>,
    /// how many artists to include.
    pub artists_count: Option<u32>,
    /// how many composers to include.
    pub composers_count: Option<u32>,
    /// how many albums to include.
    pub albums_count: Option<u32>,
    /// how many promotions to include.
    pub promotions_count: Option<u32>,
    /// how many features to include.
    pub features_count: Option<u32>,
    /// how many playlists to include.
    pub playlists_count: Option<u32>,
    /// how many concerts to include.
    pub concerts_count: Option<u32>,
    /// track sort key.
    pub tracks_sort_by: Option<String>,
    /// album sort key.
    pub albums_sort_by: Option<String>,
    /// whether to include like counts.
    pub with_likes_count: Option<bool>,
}

impl MetatagQuery {
    fn to_params(&self) -> Params {
        Params::new()
            .maybe("tracksCount", self.tracks_count)
            .maybe("artistsCount", self.artists_count)
            .maybe("composersCount", self.composers_count)
            .maybe("albumsCount", self.albums_count)
            .maybe("promotionsCount", self.promotions_count)
            .maybe("featuresCount", self.features_count)
            .maybe("playlistsCount", self.playlists_count)
            .maybe("concertsCount", self.concerts_count)
            .maybe("tracksSortBy", self.tracks_sort_by.as_deref())
            .maybe("albumsSortBy", self.albums_sort_by.as_deref())
            .maybe("withLikesCount", self.with_likes_count)
    }
}

/// pagination and sorting for the metatag listing endpoints.
///
/// `Default` gives offset 0 and limit 25, matching the api.
#[derive(Debug, Clone)]
pub struct MetatagListQuery {
    /// offset from the start.
    pub offset: u32,
    /// maximum number of items.
    pub limit: u32,
    /// time period; the artists listing requires one and defaults to `week`.
    pub period: Option<String>,
    /// sort key.
    pub sort_by: Option<String>,
    /// how many tracks to inline per artist — the artists listing only.
    pub tracks_per_artist: Option<u32>,
    /// whether to include like counts — the playlists listing only.
    pub with_likes_count: Option<bool>,
}

impl Default for MetatagListQuery {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 25,
            period: None,
            sort_by: None,
            tracks_per_artist: None,
            with_likes_count: None,
        }
    }
}

impl Client {
    /// fetch the metatag navigation: moods, activities, genres, epochs.
    ///
    /// `GET /landing3/metatags`
    pub async fn metatags(&self) -> Result<Metatags> {
        self.get("/landing3/metatags", Params::new()).await
    }

    /// fetch aggregate information about a metatag.
    ///
    /// `GET /metatags/{id}`
    pub async fn metatag(&self, metatag_id: &str, query: &MetatagQuery) -> Result<Metatag> {
        let metatag_id = segment(metatag_id);
        self.get(&format!("/metatags/{metatag_id}"), query.to_params())
            .await
    }

    /// fetch a page of a metatag's albums.
    ///
    /// `GET /metatags/{id}/albums`
    pub async fn metatag_albums(
        &self,
        metatag_id: &str,
        query: &MetatagListQuery,
    ) -> Result<MetatagAlbums> {
        let metatag_id = segment(metatag_id);
        let params = Params::new()
            .add("offset", query.offset)
            .add("limit", query.limit)
            .maybe("period", query.period.as_deref())
            .maybe("sortBy", query.sort_by.as_deref());
        self.get(&format!("/metatags/{metatag_id}/albums"), params)
            .await
    }

    /// fetch a page of a metatag's artists.
    ///
    /// `GET /metatags/{id}/artists` — the api requires a `period` here, so one
    /// defaults to `week`.
    pub async fn metatag_artists(
        &self,
        metatag_id: &str,
        query: &MetatagListQuery,
    ) -> Result<MetatagArtists> {
        let metatag_id = segment(metatag_id);
        let params = Params::new()
            .add("period", query.period.as_deref().unwrap_or("week"))
            .add("offset", query.offset)
            .add("limit", query.limit)
            .maybe("sortBy", query.sort_by.as_deref())
            .maybe("tracksPerArtist", query.tracks_per_artist);
        self.get(&format!("/metatags/{metatag_id}/artists"), params)
            .await
    }

    /// fetch a page of a metatag's playlists.
    ///
    /// `GET /metatags/{id}/playlists`
    pub async fn metatag_playlists(
        &self,
        metatag_id: &str,
        query: &MetatagListQuery,
    ) -> Result<MetatagPlaylists> {
        let metatag_id = segment(metatag_id);
        let params = Params::new()
            .add("offset", query.offset)
            .add("limit", query.limit)
            .maybe("sortBy", query.sort_by.as_deref())
            .maybe("withLikesCount", query.with_likes_count);
        self.get(&format!("/metatags/{metatag_id}/playlists"), params)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(params: &Params) -> Vec<(String, String)> {
        params
            .pairs()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn an_empty_metatag_query_sends_nothing() {
        assert!(MetatagQuery::default().to_params().is_empty());
    }

    #[test]
    fn only_the_set_metatag_counts_are_sent() {
        let query = MetatagQuery {
            tracks_count: Some(10),
            with_likes_count: Some(true),
            ..Default::default()
        };
        assert_eq!(
            collect(&query.to_params()),
            vec![
                ("tracksCount".into(), "10".into()),
                ("withLikesCount".into(), "true".into()),
            ]
        );
    }

    #[test]
    fn the_default_list_query_matches_the_api_defaults() {
        let query = MetatagListQuery::default();
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, 25);
        assert!(query.period.is_none());
    }
}
