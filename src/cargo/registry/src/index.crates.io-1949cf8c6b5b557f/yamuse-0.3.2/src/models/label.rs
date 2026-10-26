//! record-label listing pages.

use serde::{Deserialize, Serialize};

use super::Pager;
use super::album::Album;
use super::artist::Artist;

/// one page of a label's albums.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LabelAlbums {
    /// the albums on this page.
    pub albums: Vec<Album>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

/// one page of a label's artists.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LabelArtists {
    /// the artists on this page.
    pub artists: Vec<Artist>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_pages_carry_their_pager() {
        let albums: LabelAlbums = serde_json::from_str(
            r#"{"albums":[{"id":1}],"pager":{"total":50,"page":0,"perPage":25}}"#,
        )
        .unwrap();
        assert_eq!(albums.albums.len(), 1);
        assert!(albums.pager.unwrap().has_next());
    }

    #[test]
    fn label_models_survive_an_empty_object() {
        serde_json::from_str::<LabelAlbums>("{}").unwrap();
        serde_json::from_str::<LabelArtists>("{}").unwrap();
    }
}
