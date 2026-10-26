//! the clip (short music video) model.

use serde::{Deserialize, Serialize};

use super::artist::Artist;
use super::{ContentRestrictions, Cover, Pager};

/// a short music video.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Clip {
    /// clip identifier.
    pub clip_id: Option<i64>,
    /// clip title.
    pub title: Option<String>,
    /// version label, for example "official video".
    pub version: Option<String>,
    /// player identifier.
    pub player_id: Option<String>,
    /// stable uuid.
    pub uuid: Option<String>,
    /// thumbnail uri.
    pub thumbnail: Option<String>,
    /// preview stream url.
    pub preview_url: Option<String>,
    /// duration in seconds.
    pub duration: Option<i64>,
    /// ids of the tracks the clip is associated with.
    pub track_ids: Vec<i64>,
    /// performing artists.
    pub artists: Vec<Artist>,
    /// disclaimers.
    pub disclaimers: Vec<String>,
    /// whether the clip is explicit.
    pub explicit: Option<bool>,
    /// cover art.
    pub cover: Option<Cover>,
    /// content availability restrictions.
    pub content_restrictions: Option<ContentRestrictions>,
}

/// a recommended page of clips.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipsWillLike {
    /// the recommended clips.
    pub clips: Vec<Clip>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_deserialize_with_their_artists() {
        let page: ClipsWillLike = serde_json::from_str(
            r#"{"clips":[{"clipId":7,"title":"c","artists":[{"name":"A"}]}],"pager":{"total":1}}"#,
        )
        .unwrap();
        assert_eq!(page.clips[0].clip_id, Some(7));
        assert_eq!(page.clips[0].artists[0].name.as_deref(), Some("A"));
        assert_eq!(page.pager.unwrap().total, Some(1));
    }

    #[test]
    fn clip_models_survive_an_empty_object() {
        serde_json::from_str::<Clip>("{}").unwrap();
        serde_json::from_str::<ClipsWillLike>("{}").unwrap();
    }
}
