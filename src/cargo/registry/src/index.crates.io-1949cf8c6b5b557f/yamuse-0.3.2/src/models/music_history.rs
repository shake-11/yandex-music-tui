//! listening history, grouped by day and by playback context.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Id;

/// identifiers carried by a history item.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryItemId {
    /// generic id.
    pub id: Option<String>,
    /// track id.
    pub track_id: Option<String>,
    /// album id.
    pub album_id: Option<String>,
    /// owner uid, for playlists and waves.
    pub uid: Option<Id>,
    /// playlist kind.
    pub kind: Option<Id>,
    /// wave seeds.
    pub seeds: Vec<String>,
}

/// the payload of a single history item.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryItemData {
    /// the item identifiers.
    pub item_id: Option<MusicHistoryItemId>,
    /// the full entity, present only when `full_models_count` asked for it.
    pub full_model: Option<Value>,
}

/// a typed entry within a history group: a context or a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryItem {
    /// entry type, for example `track` or a context kind.
    pub r#type: Option<String>,
    /// the entry payload.
    pub data: Option<MusicHistoryItemData>,
}

/// a context together with the tracks played in it.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryGroup {
    /// the playback context.
    pub context: Option<MusicHistoryItem>,
    /// tracks played within the context.
    pub tracks: Vec<MusicHistoryItem>,
}

/// a day's worth of listening history.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryTab {
    /// date of the tab.
    pub date: Option<String>,
    /// history groups for the day.
    pub items: Vec<MusicHistoryGroup>,
}

/// the full listening history, grouped by day.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistory {
    /// history tabs, one per day.
    pub history_tabs: Vec<MusicHistoryTab>,
}

/// the result of resolving specific history items.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MusicHistoryItems {
    /// the resolved items.
    pub items: Vec<MusicHistoryItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_nests_days_then_contexts_then_tracks() {
        let history: MusicHistory = serde_json::from_str(
            r#"{"historyTabs":[{"date":"2026-07-30","items":[{
                "context":{"type":"playlist","data":{"itemId":{"uid":1,"kind":"1000"}}},
                "tracks":[{"type":"track","data":{"itemId":{"trackId":"42","albumId":"7"}}}]
            }]}]}"#,
        )
        .unwrap();

        let tab = &history.history_tabs[0];
        assert_eq!(tab.date.as_deref(), Some("2026-07-30"));
        let group = &tab.items[0];
        let context_id = group
            .context
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .unwrap()
            .item_id
            .as_ref()
            .unwrap();
        // uid arrives numeric, kind quoted — both normalize
        assert_eq!(context_id.uid.as_ref().unwrap().to_string(), "1");
        assert_eq!(context_id.kind.as_ref().unwrap().to_string(), "1000");
        assert_eq!(group.tracks.len(), 1);
    }

    #[test]
    fn full_models_are_kept_when_requested() {
        let item: MusicHistoryItem = serde_json::from_str(
            r#"{"type":"track","data":{"itemId":{"trackId":"1"},"fullModel":{"title":"T"}}}"#,
        )
        .unwrap();
        assert!(item.data.unwrap().full_model.is_some());
    }

    #[test]
    fn history_models_survive_an_empty_object() {
        serde_json::from_str::<MusicHistory>("{}").unwrap();
        serde_json::from_str::<MusicHistoryItems>("{}").unwrap();
        serde_json::from_str::<MusicHistoryTab>("{}").unwrap();
        serde_json::from_str::<MusicHistoryGroup>("{}").unwrap();
        serde_json::from_str::<MusicHistoryItem>("{}").unwrap();
        serde_json::from_str::<MusicHistoryItemId>("{}").unwrap();
    }
}
