//! supplementary track information: legacy lyrics and associated videos.

use serde::{Deserialize, Serialize};

/// track lyrics embedded in the supplement.
///
/// this is the legacy path. prefer the dedicated lyrics endpoint
/// (`Client::track_lyrics`), which also serves time-synced lrc.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Lyrics {
    /// lyric id.
    pub id: Option<i64>,
    /// lyrics, possibly truncated.
    pub lyrics: Option<String>,
    /// full lyrics.
    pub full_lyrics: Option<String>,
    /// whether the lyrics are rights-cleared.
    pub has_rights: Option<bool>,
    /// whether a translation is shown.
    pub show_translation: Option<bool>,
    /// lyrics language.
    pub text_language: Option<String>,
    /// source url.
    pub url: Option<String>,
}

/// a video associated with a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoSupplement {
    /// cover url.
    pub cover: Option<String>,
    /// hosting provider.
    pub provider: Option<String>,
    /// title.
    pub title: Option<String>,
    /// provider video id.
    pub provider_video_id: Option<String>,
    /// direct url.
    pub url: Option<String>,
    /// embeddable url.
    pub embed_url: Option<String>,
    /// embed markup.
    pub embed: Option<String>,
}

/// supplementary information about a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Supplement {
    /// supplement id.
    pub id: Option<i64>,
    /// embedded lyrics.
    pub lyrics: Option<Lyrics>,
    /// associated videos.
    pub videos: Vec<VideoSupplement>,
    /// whether a radio is available for the track.
    pub radio_is_available: Option<bool>,
    /// description.
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_supplement_carries_lyrics_and_videos() {
        let supplement: Supplement = serde_json::from_str(
            r#"{"id":1,"lyrics":{"fullLyrics":"line one\nline two","hasRights":true},"videos":[{"provider":"youtube"}]}"#,
        )
        .unwrap();
        assert!(
            supplement
                .lyrics
                .unwrap()
                .full_lyrics
                .unwrap()
                .contains("line two")
        );
        assert_eq!(supplement.videos[0].provider.as_deref(), Some("youtube"));
    }

    #[test]
    fn supplement_models_survive_an_empty_object() {
        serde_json::from_str::<Supplement>("{}").unwrap();
        serde_json::from_str::<Lyrics>("{}").unwrap();
        serde_json::from_str::<VideoSupplement>("{}").unwrap();
    }
}
