//! the music-video model.

use serde::{Deserialize, Serialize};

use super::Id;

/// a music video, typically hosted on an external provider.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Video {
    /// video title.
    pub title: Option<String>,
    /// cover image url.
    pub cover: Option<String>,
    /// embeddable player url.
    pub embed_url: Option<String>,
    /// hosting provider, for example `youtube`.
    pub provider: Option<String>,
    /// provider-specific video id.
    pub provider_video_id: Option<Id>,
    /// youtube url, when applicable.
    pub youtube_url: Option<String>,
    /// thumbnail url.
    pub thumbnail_url: Option<String>,
    /// duration in seconds.
    pub duration: Option<i64>,
    /// free-form text.
    pub text: Option<String>,
    /// auto-playing html5 player markup.
    pub html_auto_play_video_player: Option<String>,
    /// regions where available.
    pub regions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_ids_accept_both_json_shapes() {
        let numeric: Video = serde_json::from_str(r#"{"providerVideoId":42}"#).unwrap();
        let text: Video = serde_json::from_str(r#"{"providerVideoId":"dQw4w9WgXcQ"}"#).unwrap();
        assert_eq!(numeric.provider_video_id.unwrap().to_string(), "42");
        assert_eq!(text.provider_video_id.unwrap().to_string(), "dQw4w9WgXcQ");
    }

    #[test]
    fn video_survives_an_empty_object() {
        serde_json::from_str::<Video>("{}").unwrap();
    }
}
