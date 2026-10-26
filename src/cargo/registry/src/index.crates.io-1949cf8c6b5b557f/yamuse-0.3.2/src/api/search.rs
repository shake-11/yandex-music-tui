//! search endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Search, Suggestions};
use crate::params::Params;
use crate::types::SearchType;

/// options for [`Client::search`].
///
/// `Default` gives the api's own defaults: every entity type, first page,
/// typo-correction on, playlists eligible as the best match.
#[derive(Debug, Clone, Copy)]
pub struct SearchQuery {
    /// entity type to search for.
    pub r#type: SearchType,
    /// page index, zero-based.
    pub page: u32,
    /// set `true` to suppress typo correction.
    pub nocorrect: bool,
    /// whether playlists may appear as the best match.
    pub playlist_in_best: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            r#type: SearchType::All,
            page: 0,
            nocorrect: false,
            playlist_in_best: true,
        }
    }
}

impl SearchQuery {
    /// restrict the search to one entity type.
    #[must_use]
    pub fn of_type(mut self, r#type: SearchType) -> Self {
        self.r#type = r#type;
        self
    }

    /// request a specific page.
    #[must_use]
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// search the query verbatim, without typo correction.
    #[must_use]
    pub fn exact(mut self) -> Self {
        self.nocorrect = true;
        self
    }
}

impl Client {
    /// search the catalogue.
    ///
    /// `GET /search`
    ///
    /// # examples
    ///
    /// ```no_run
    /// # async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
    /// use yamuse::{SearchQuery, SearchType};
    ///
    /// let tracks_only = client
    ///     .search("bohemian rhapsody", SearchQuery::default().of_type(SearchType::Track))
    ///     .await?;
    /// println!("{} matches", tracks_only.track_results().len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(&self, text: &str, query: SearchQuery) -> Result<Search> {
        let params = Params::new()
            .add("text", text)
            .add("nocorrect", query.nocorrect)
            .add("type", query.r#type)
            .add("page", query.page)
            .add("playlist-in-best", query.playlist_in_best);
        self.get("/search", params).await
    }

    /// fetch search suggestions for a partial query.
    ///
    /// `GET /search/suggest`
    pub async fn search_suggest(&self, part: &str) -> Result<Suggestions> {
        self.get("/search/suggest", Params::new().add("part", part))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_query_matches_the_api_defaults() {
        let query = SearchQuery::default();
        assert_eq!(query.r#type, SearchType::All);
        assert_eq!(query.page, 0);
        assert!(!query.nocorrect);
        assert!(query.playlist_in_best);
    }

    #[test]
    fn builder_methods_chain() {
        let query = SearchQuery::default()
            .of_type(SearchType::Album)
            .page(3)
            .exact();
        assert_eq!(query.r#type, SearchType::Album);
        assert_eq!(query.page, 3);
        assert!(query.nocorrect);
    }
}
