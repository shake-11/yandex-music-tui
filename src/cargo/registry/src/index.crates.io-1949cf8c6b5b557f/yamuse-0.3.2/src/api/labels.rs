//! record-label endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Label, LabelAlbums, LabelArtists};
use crate::params::{Params, segment};
use crate::types::{SortBy, SortOrder};

impl Client {
    /// fetch a record label by id.
    ///
    /// `GET /labels/{id}`
    pub async fn label(&self, label_id: impl std::fmt::Display) -> Result<Label> {
        let label_id = segment(label_id);
        self.get(&format!("/labels/{label_id}"), Params::new())
            .await
    }

    /// fetch a page of a label's albums.
    ///
    /// `GET /labels/{id}/albums` — note the camelCase `pageSize`, unlike the
    /// hyphenated `page-size` the artist listings use.
    pub async fn label_albums(
        &self,
        label_id: impl std::fmt::Display,
        page: u32,
        page_size: u32,
        sort_by: Option<SortBy>,
        sort_order: Option<SortOrder>,
    ) -> Result<LabelAlbums> {
        let label_id = segment(label_id);
        let params = Params::new()
            .add("page", page)
            .add("pageSize", page_size)
            .maybe("sortBy", sort_by)
            .maybe("sortOrder", sort_order);
        self.get(&format!("/labels/{label_id}/albums"), params)
            .await
    }

    /// fetch a page of a label's artists.
    ///
    /// `GET /labels/{id}/artists`
    pub async fn label_artists(
        &self,
        label_id: impl std::fmt::Display,
        page: u32,
        page_size: u32,
    ) -> Result<LabelArtists> {
        let label_id = segment(label_id);
        let params = Params::new().add("page", page).add("pageSize", page_size);
        self.get(&format!("/labels/{label_id}/artists"), params)
            .await
    }
}
