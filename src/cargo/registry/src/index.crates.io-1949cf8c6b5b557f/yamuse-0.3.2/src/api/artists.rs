//! artist endpoints.

use crate::api::MAX_PAGES;
use crate::client::Client;
use crate::error::Result;
use crate::models::{
    Album, Artist, ArtistAbout, ArtistAlbums, ArtistClips, ArtistDonations, ArtistInfo,
    ArtistLinks, ArtistSimilar, ArtistSkeleton, ArtistTracks, ArtistTrailer, BriefInfo, Track,
};
use crate::params::{Params, segment};
use crate::types::{BatchObject, SortBy, SortOrder};

/// pagination and sorting for the artist-album listings.
///
/// `Default` gives the api's own defaults: first page, 20 per page, newest first.
#[derive(Debug, Clone, Copy)]
pub struct ArtistAlbumsQuery {
    /// page index, zero-based.
    pub page: u32,
    /// albums per page.
    pub page_size: u32,
    /// sort key.
    pub sort_by: SortBy,
    /// sort direction. only the safe-direct listing honours this.
    pub sort_order: SortOrder,
}

impl Default for ArtistAlbumsQuery {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 20,
            sort_by: SortBy::default(),
            sort_order: SortOrder::default(),
        }
    }
}

impl ArtistAlbumsQuery {
    /// request a specific page.
    #[must_use]
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// set the page size.
    #[must_use]
    pub fn page_size(mut self, page_size: u32) -> Self {
        self.page_size = page_size;
        self
    }

    /// set the sort key.
    #[must_use]
    pub fn sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    fn to_params(self) -> Params {
        Params::new()
            .add("sort-by", self.sort_by)
            .add("page", self.page)
            .add("page-size", self.page_size)
    }
}

impl Client {
    /// fetch one or many artists by id.
    ///
    /// `POST /artists`
    pub async fn artists(
        &self,
        artist_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Vec<Artist>> {
        self.get_batch(BatchObject::Artist, artist_ids, Params::new())
            .await
    }

    /// fetch one artist by id, or `None` when it does not resolve.
    pub async fn artist(&self, artist_id: impl std::fmt::Display) -> Result<Option<Artist>> {
        Ok(self.artists([artist_id]).await?.into_iter().next())
    }

    /// fetch aggregate information about an artist — everything the artist page shows.
    ///
    /// `GET /artists/{id}/brief-info`
    pub async fn artist_brief_info(&self, artist_id: impl std::fmt::Display) -> Result<BriefInfo> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/brief-info"), Params::new())
            .await
    }

    /// fetch a page of an artist's tracks.
    ///
    /// `GET /artists/{id}/tracks`
    pub async fn artist_tracks(
        &self,
        artist_id: impl std::fmt::Display,
        page: u32,
        page_size: u32,
    ) -> Result<ArtistTracks> {
        let artist_id = segment(artist_id);
        let params = Params::new().add("page", page).add("page-size", page_size);
        self.get(&format!("/artists/{artist_id}/tracks"), params)
            .await
    }

    /// fetch every track by an artist, walking the pages.
    ///
    /// a convenience over [`artist_tracks`](Self::artist_tracks): the requests are
    /// sequential, so a prolific artist costs several round trips. stops after
    /// 100 pages regardless of what the pager claims, so a misreported pager
    /// cannot loop forever.
    pub async fn artist_tracks_all(
        &self,
        artist_id: impl std::fmt::Display,
        page_size: u32,
    ) -> Result<Vec<Track>> {
        let artist_id = artist_id.to_string();
        let mut all = Vec::new();
        for page in 0..MAX_PAGES {
            let batch = self.artist_tracks(&artist_id, page, page_size).await?;
            all.extend(batch.tracks);
            if !batch.pager.is_some_and(|pager| pager.has_next_after(page)) {
                break;
            }
        }
        Ok(all)
    }

    /// fetch a page of albums where the artist is the primary author.
    ///
    /// `GET /artists/{id}/direct-albums`
    pub async fn artist_direct_albums(
        &self,
        artist_id: impl std::fmt::Display,
        query: ArtistAlbumsQuery,
    ) -> Result<ArtistAlbums> {
        let artist_id = segment(artist_id);
        self.get(
            &format!("/artists/{artist_id}/direct-albums"),
            query.to_params(),
        )
        .await
    }

    /// fetch every album where the artist is the primary author, walking the pages.
    ///
    /// see [`artist_tracks_all`](Self::artist_tracks_all) for the caveats.
    pub async fn artist_direct_albums_all(
        &self,
        artist_id: impl std::fmt::Display,
        query: ArtistAlbumsQuery,
    ) -> Result<Vec<Album>> {
        let artist_id = artist_id.to_string();
        let mut all = Vec::new();
        for page in 0..MAX_PAGES {
            let batch = self
                .artist_direct_albums(&artist_id, query.page(page))
                .await?;
            all.extend(batch.albums);
            if !batch.pager.is_some_and(|pager| pager.has_next_after(page)) {
                break;
            }
        }
        Ok(all)
    }

    /// fetch a page of compilations and albums the artist also appears on.
    ///
    /// `GET /artists/{id}/also-albums`
    pub async fn artist_also_albums(
        &self,
        artist_id: impl std::fmt::Display,
        query: ArtistAlbumsQuery,
    ) -> Result<ArtistAlbums> {
        let artist_id = segment(artist_id);
        self.get(
            &format!("/artists/{artist_id}/also-albums"),
            query.to_params(),
        )
        .await
    }

    /// fetch a page of the artist's discography.
    ///
    /// `GET /artists/{id}/discography-albums`
    pub async fn artist_discography_albums(
        &self,
        artist_id: impl std::fmt::Display,
        query: ArtistAlbumsQuery,
    ) -> Result<ArtistAlbums> {
        let artist_id = segment(artist_id);
        self.get(
            &format!("/artists/{artist_id}/discography-albums"),
            query.to_params(),
        )
        .await
    }

    /// fetch the artist's "safe" direct albums — no explicit content.
    ///
    /// `GET /artists/{id}/safe-direct-albums`
    ///
    /// this listing takes `limit` and `sort-order` rather than page/page-size.
    pub async fn artist_safe_direct_albums(
        &self,
        artist_id: impl std::fmt::Display,
        query: ArtistAlbumsQuery,
    ) -> Result<ArtistAlbums> {
        let artist_id = segment(artist_id);
        let params = Params::new()
            .add("sort-by", query.sort_by)
            .add("sort-order", query.sort_order)
            .add("limit", query.page_size);
        self.get(&format!("/artists/{artist_id}/safe-direct-albums"), params)
            .await
    }

    /// fetch artists similar to the given one.
    ///
    /// `GET /artists/{id}/similar`
    pub async fn artist_similar(&self, artist_id: impl std::fmt::Display) -> Result<ArtistSimilar> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/similar"), Params::new())
            .await
    }

    /// fetch the external links shown on an artist's page.
    ///
    /// `GET /artists/{id}/artist-links`
    pub async fn artist_links(&self, artist_id: impl std::fmt::Display) -> Result<ArtistLinks> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/artist-links"), Params::new())
            .await
    }

    /// fetch the trailer of an artist.
    ///
    /// `GET /artists/{id}/trailer`
    pub async fn artist_trailer(&self, artist_id: impl std::fmt::Display) -> Result<ArtistTrailer> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/trailer"), Params::new())
            .await
    }

    /// fetch a page of the artist's track ids.
    ///
    /// `GET /artists/{id}/track-ids`
    pub async fn artist_track_ids(
        &self,
        artist_id: impl std::fmt::Display,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<i64>> {
        let artist_id = segment(artist_id);
        let params = Params::new().add("page", page).add("page-size", page_size);
        let value = self
            .get_value(&format!("/artists/{artist_id}/track-ids"), params)
            .await?;
        Ok(value
            .as_array()
            .map(|items| items.iter().filter_map(serde_json::Value::as_i64).collect())
            .unwrap_or_default())
    }

    /// fetch detailed information about an artist.
    ///
    /// `GET /artists/{id}/info`
    pub async fn artist_info(&self, artist_id: impl std::fmt::Display) -> Result<ArtistInfo> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/info"), Params::new())
            .await
    }

    /// fetch the "about" block of an artist.
    ///
    /// `GET /artists/{id}/about-artist`
    pub async fn artist_about(&self, artist_id: impl std::fmt::Display) -> Result<ArtistAbout> {
        let artist_id = segment(artist_id);
        self.get(&format!("/artists/{artist_id}/about-artist"), Params::new())
            .await
    }

    /// fetch the artist-clips block of an artist.
    ///
    /// `GET /artists/{id}/blocks/artist-clips`
    pub async fn artist_clips(&self, artist_id: impl std::fmt::Display) -> Result<ArtistClips> {
        let artist_id = segment(artist_id);
        self.get(
            &format!("/artists/{artist_id}/blocks/artist-clips"),
            Params::new(),
        )
        .await
    }

    /// fetch the artist-donation block of an artist.
    ///
    /// `GET /artists/{id}/blocks/artist-donation`
    pub async fn artist_donation(
        &self,
        artist_id: impl std::fmt::Display,
    ) -> Result<ArtistDonations> {
        let artist_id = segment(artist_id);
        self.get(
            &format!("/artists/{artist_id}/blocks/artist-donation"),
            Params::new(),
        )
        .await
    }

    /// fetch a page-layout skeleton of an artist.
    ///
    /// `GET /artists/{id}/skeletons/{skeleton_id}`
    pub async fn artist_skeleton(
        &self,
        artist_id: impl std::fmt::Display,
        skeleton_id: &str,
    ) -> Result<ArtistSkeleton> {
        let artist_id = segment(artist_id);
        let skeleton_id = segment(skeleton_id);
        self.get(
            &format!("/artists/{artist_id}/skeletons/{skeleton_id}"),
            Params::new(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_album_query_matches_the_api_defaults() {
        let query = ArtistAlbumsQuery::default();
        assert_eq!(query.page, 0);
        assert_eq!(query.page_size, 20);
        assert_eq!(query.sort_by, SortBy::Year);
        assert_eq!(query.sort_order, SortOrder::Desc);
    }

    #[test]
    fn album_query_params_use_the_hyphenated_keys_the_api_expects() {
        let params = ArtistAlbumsQuery::default()
            .page(2)
            .page_size(50)
            .sort_by(SortBy::Rating)
            .to_params();
        let pairs: Vec<(String, String)> = params
            .pairs()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("sort-by".into(), "rating".into()),
                ("page".into(), "2".into()),
                ("page-size".into(), "50".into()),
            ]
        );
    }
}
