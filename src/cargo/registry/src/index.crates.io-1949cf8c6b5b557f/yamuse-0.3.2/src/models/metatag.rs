//! metatag navigation — moods, activities, genres and epochs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Pager;
use super::album::Album;
use super::artist::Artist;
use super::playlist::Playlist;
use super::track::Track;

/// a metatag title, in short and full forms.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagTitle {
    /// short title.
    pub title: Option<String>,
    /// full title.
    pub full_title: Option<String>,
}

/// a selectable sort option for a metatag listing.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagSortByValue {
    /// sort value; `popular` and `new` are the known values.
    pub value: Option<String>,
    /// human-readable label.
    pub title: Option<String>,
    /// whether this sort is currently active.
    pub active: Option<bool>,
}

/// a leaf in a metatag navigation tree, which may nest further leaves.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagLeaf {
    /// tag id, used as the metatag identifier in requests.
    pub tag: Option<String>,
    /// leaf title.
    pub title: Option<String>,
    /// nested leaves.
    pub leaves: Vec<MetatagLeaf>,
}

/// a metatag navigation tree, for example "moods" or "genres".
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagTree {
    /// tree title.
    pub title: Option<String>,
    /// navigation id: `moods`, `activities`, `genres` or `epochs`.
    pub navigation_id: Option<String>,
    /// top-level leaves.
    pub leaves: Vec<MetatagLeaf>,
}

/// the metatag navigation, as a list of trees.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Metatags {
    /// the navigation trees.
    pub trees: Vec<MetatagTree>,
}

/// an artist together with a few of their popular tracks, inside a metatag.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagArtistEntry {
    /// the artist.
    pub artist: Option<Artist>,
    /// a selection of popular tracks.
    pub popular_tracks: Vec<Track>,
}

/// aggregate metatag information.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Metatag {
    /// metatag id.
    pub id: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// accent color.
    pub color: Option<String>,
    /// title.
    pub title: Option<MetatagTitle>,
    /// whether the metatag is liked.
    pub liked: Option<bool>,
    /// radio station id for the metatag.
    pub station_id: Option<String>,
    /// custom wave animation url.
    pub custom_wave_animation_url: Option<String>,
    /// featured artists.
    pub artists: Vec<Artist>,
    /// featured albums.
    pub albums: Vec<Album>,
    /// featured playlists.
    pub playlists: Vec<Playlist>,
    /// available track sort options.
    pub tracks_sort_by_values: Vec<MetatagSortByValue>,
    /// available album sort options.
    pub albums_sort_by_values: Vec<MetatagSortByValue>,
    /// available playlist sort options.
    pub playlists_sort_by_values: Vec<MetatagSortByValue>,
    /// featured composers.
    pub composers: Vec<Artist>,
    /// featured tracks.
    pub tracks: Vec<Track>,
    /// featured entities, still free-form upstream.
    pub features: Option<Value>,
    /// promotions, still free-form upstream.
    pub promotions: Option<Value>,
}

/// one page of a metatag's albums.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagAlbums {
    /// metatag id.
    pub id: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// accent color.
    pub color: Option<String>,
    /// title.
    pub title: Option<MetatagTitle>,
    /// radio station id.
    pub station_id: Option<String>,
    /// pagination metadata.
    pub pager: Option<Pager>,
    /// the albums on this page.
    pub albums: Vec<Album>,
    /// available sort options.
    pub sort_by_values: Vec<MetatagSortByValue>,
}

/// one page of a metatag's artists.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagArtists {
    /// metatag id.
    pub id: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// accent color.
    pub color: Option<String>,
    /// title.
    pub title: Option<MetatagTitle>,
    /// radio station id.
    pub station_id: Option<String>,
    /// pagination metadata.
    pub pager: Option<Pager>,
    /// the artist entries on this page.
    pub artists: Vec<MetatagArtistEntry>,
    /// available sort options.
    pub sort_by_values: Vec<MetatagSortByValue>,
}

/// one page of a metatag's playlists.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetatagPlaylists {
    /// metatag id.
    pub id: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// accent color.
    pub color: Option<String>,
    /// title.
    pub title: Option<MetatagTitle>,
    /// radio station id.
    pub station_id: Option<String>,
    /// pagination metadata.
    pub pager: Option<Pager>,
    /// the playlists on this page.
    pub playlists: Vec<Playlist>,
    /// available sort options.
    pub sort_by_values: Vec<MetatagSortByValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_leaves_nest_recursively() {
        let tags: Metatags = serde_json::from_str(
            r#"{"trees":[{"navigationId":"moods","leaves":[{"tag":"happy","leaves":[{"tag":"upbeat"}]}]}]}"#,
        )
        .unwrap();
        let tree = &tags.trees[0];
        assert_eq!(tree.navigation_id.as_deref(), Some("moods"));
        assert_eq!(tree.leaves[0].leaves[0].tag.as_deref(), Some("upbeat"));
    }

    #[test]
    fn metatag_models_survive_an_empty_object() {
        serde_json::from_str::<Metatags>("{}").unwrap();
        serde_json::from_str::<Metatag>("{}").unwrap();
        serde_json::from_str::<MetatagAlbums>("{}").unwrap();
        serde_json::from_str::<MetatagArtists>("{}").unwrap();
        serde_json::from_str::<MetatagPlaylists>("{}").unwrap();
        serde_json::from_str::<MetatagTree>("{}").unwrap();
        serde_json::from_str::<MetatagLeaf>("{}").unwrap();
        serde_json::from_str::<MetatagTitle>("{}").unwrap();
        serde_json::from_str::<MetatagSortByValue>("{}").unwrap();
        serde_json::from_str::<MetatagArtistEntry>("{}").unwrap();
    }
}
