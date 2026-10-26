//! small standalone models: mutation acknowledgements, trailers and tag results.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::track::Track;

/// what a mutation endpoint answered.
///
/// every mutating method returns this rather than a `bool`. a `bool` could only
/// ever say "the api did not acknowledge that", which callers do not check and
/// which discards the reason — an unacknowledged mutation is an `Err` now, so it
/// cannot pass silently.
///
/// the type is not empty because two of those endpoints do report something back:
/// track like and dislike answer with the library revision instead of the bare
/// `"ok"` everything else uses.
///
/// ```
/// # async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
/// let done = client.like_tracks([42]).await?;
/// if let Some(revision) = done.revision {
///     println!("library is now at revision {revision}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct Mutation {
    /// the library revision after the change, when the endpoint reports one.
    ///
    /// only the track like and dislike endpoints do. this is the *library*
    /// revision, unrelated to the playlist revision
    /// [`Client::change_playlist`](crate::Client::change_playlist) takes.
    pub revision: Option<i64>,
}

/// a trailer: a titled selection of tracks.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrailerInfo {
    /// trailer title.
    pub title: Option<String>,
    /// trailer tracks.
    pub tracks: Vec<Track>,
}

/// the playlists grouped under a tag.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TagResult {
    /// the tag descriptor, still free-form upstream.
    pub tag: Option<Value>,
    /// ids of the tagged playlists, as `uid:kind` strings.
    pub ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misc_models_survive_an_empty_object() {
        serde_json::from_str::<TrailerInfo>("{}").unwrap();
        serde_json::from_str::<TagResult>("{}").unwrap();
    }

    #[test]
    fn tag_results_carry_playlist_ids() {
        let result: TagResult =
            serde_json::from_str(r#"{"tag":{"id":"rock"},"ids":["1:2","3:4"]}"#).unwrap();
        assert_eq!(result.ids, vec!["1:2", "3:4"]);
        assert!(result.tag.is_some());
    }
}
