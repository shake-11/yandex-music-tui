//! playback queues.

use serde::{Deserialize, Serialize};

use super::track::TrackId;

/// the context a queue was started from: album, playlist, radio, ...
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Context {
    /// context type.
    pub r#type: Option<String>,
    /// context id.
    pub id: Option<String>,
    /// context description.
    pub description: Option<String>,
}

/// a summary entry in the list of a user's queues.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QueueItem {
    /// queue id.
    pub id: Option<String>,
    /// queue context.
    pub context: Option<Context>,
    /// last modification timestamp.
    pub modified: Option<String>,
}

/// a full playback queue with its tracks and current position.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Queue {
    /// the context the queue was started from.
    pub context: Option<Context>,
    /// track references in the queue.
    pub tracks: Vec<TrackId>,
    /// index of the currently playing track.
    pub current_index: Option<i64>,
    /// last modification timestamp.
    pub modified: Option<String>,
    /// queue id.
    pub id: Option<String>,
    /// origin tag used in `from` parameters.
    pub from: Option<String>,
}

impl Queue {
    /// the track reference currently playing, when the index is in range.
    #[must_use]
    pub fn current_track(&self) -> Option<&TrackId> {
        let index = usize::try_from(self.current_index?).ok()?;
        self.tracks.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_track_is_resolved_from_the_index() {
        let queue: Queue = serde_json::from_str(
            r#"{"tracks":[{"trackId":1},{"trackId":2},{"trackId":3}],"currentIndex":1}"#,
        )
        .unwrap();
        assert_eq!(queue.current_track().unwrap().track_id, Some(2));
    }

    #[test]
    fn an_out_of_range_or_negative_index_resolves_to_nothing() {
        let past_end: Queue =
            serde_json::from_str(r#"{"tracks":[{"trackId":1}],"currentIndex":9}"#).unwrap();
        assert!(past_end.current_track().is_none());

        let negative: Queue =
            serde_json::from_str(r#"{"tracks":[{"trackId":1}],"currentIndex":-1}"#).unwrap();
        assert!(negative.current_track().is_none());

        assert!(Queue::default().current_track().is_none());
    }

    #[test]
    fn queue_models_survive_an_empty_object() {
        serde_json::from_str::<Queue>("{}").unwrap();
        serde_json::from_str::<QueueItem>("{}").unwrap();
        serde_json::from_str::<Context>("{}").unwrap();
    }
}
