//! alice shots — the short spoken segments played between tracks.

use serde::{Deserialize, Serialize};

/// the type or category of a shot.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShotType {
    /// type id.
    pub id: Option<String>,
    /// type title.
    pub title: Option<String>,
}

/// the payload of a shot: cover, audio and text.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShotData {
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// audio url.
    pub mds_url: Option<String>,
    /// spoken text.
    pub shot_text: Option<String>,
    /// shot type.
    pub shot_type: Option<ShotType>,
}

/// a single shot.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Shot {
    /// playback order.
    pub order: Option<i64>,
    /// whether already played.
    pub played: Option<bool>,
    /// shot payload.
    pub shot_data: Option<ShotData>,
    /// shot id.
    pub shot_id: Option<String>,
    /// shot status.
    pub status: Option<String>,
}

/// an event carrying shots to play after a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShotEvent {
    /// event id.
    pub event_id: Option<String>,
    /// the shots to play.
    pub shots: Vec<Shot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_event_deserializes_with_its_payload() {
        let event: ShotEvent = serde_json::from_str(
            r#"{"eventId":"e1","shots":[{"order":1,"shotId":"s1","shotData":{"shotText":"hi","shotType":{"id":"alice"}}}]}"#,
        )
        .unwrap();
        let data = event.shots[0].shot_data.as_ref().unwrap();
        assert_eq!(data.shot_text.as_deref(), Some("hi"));
        assert_eq!(
            data.shot_type.as_ref().unwrap().id.as_deref(),
            Some("alice")
        );
    }

    #[test]
    fn shot_models_survive_an_empty_object() {
        serde_json::from_str::<ShotEvent>("{}").unwrap();
        serde_json::from_str::<Shot>("{}").unwrap();
        serde_json::from_str::<ShotData>("{}").unwrap();
        serde_json::from_str::<ShotType>("{}").unwrap();
    }
}
