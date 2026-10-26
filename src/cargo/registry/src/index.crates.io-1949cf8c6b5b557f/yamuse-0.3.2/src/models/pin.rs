//! pinned profile items.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ContentRestrictions, Cover};

/// the payload of a [`Pin`]: an album, artist, playlist or wave reference.
///
/// which fields are populated depends on the pin's [`r#type`](Pin::type).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinData {
    /// entity id, for albums and artists.
    pub id: Option<i64>,
    /// owner uid, for playlists.
    pub uid: Option<i64>,
    /// playlist kind, for playlists.
    pub kind: Option<i64>,
    /// playlist uuid, for playlists only.
    pub playlist_uuid: Option<String>,
    /// entity name, for artists.
    pub name: Option<String>,
    /// entity title, for albums and playlists.
    pub title: Option<String>,
    /// cover art.
    pub cover: Option<Cover>,
    /// content availability restrictions.
    pub content_restrictions: Option<ContentRestrictions>,
    /// station id, for wave pins.
    pub station_id: Option<String>,
    /// wave agent, for wave pins.
    pub agent: Option<String>,
    /// animated cover url, for wave pins.
    pub animation_url: Option<String>,
    /// background image uri template, for wave pins.
    pub background_image_url: Option<String>,
    /// content warning marker, for wave pins.
    pub content_warning: Option<String>,
    /// theme colors, still free-form upstream.
    pub colors: Option<Value>,
    /// header descriptor, still free-form upstream.
    pub header: Option<Value>,
    /// wave seeds, still free-form upstream.
    pub seeds: Option<Value>,
}

/// a pinned item on a user's profile.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Pin {
    /// pin type: `artist_item`, `album_item`, `playlist_item` or `wave_item`.
    pub r#type: Option<String>,
    /// the pinned entity.
    pub data: Option<PinData>,
}

/// the list of a user's pinned items.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PinsList {
    /// the pinned items.
    pub pins: Vec<Pin>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pin_kind_populates_its_own_fields() {
        let list: PinsList = serde_json::from_str(
            r#"{"pins":[
                {"type":"album_item","data":{"id":1,"title":"A"}},
                {"type":"artist_item","data":{"id":2,"name":"B"}},
                {"type":"playlist_item","data":{"uid":3,"kind":4}},
                {"type":"wave_item","data":{"stationId":"user:onyourwave","seeds":["x"]}}
            ]}"#,
        )
        .unwrap();
        assert_eq!(list.pins.len(), 4);
        assert_eq!(
            list.pins[0].data.as_ref().unwrap().title.as_deref(),
            Some("A")
        );
        assert_eq!(
            list.pins[1].data.as_ref().unwrap().name.as_deref(),
            Some("B")
        );
        assert_eq!(list.pins[2].data.as_ref().unwrap().kind, Some(4));
        assert!(list.pins[3].data.as_ref().unwrap().seeds.is_some());
    }

    #[test]
    fn pin_models_survive_an_empty_object() {
        serde_json::from_str::<Pin>("{}").unwrap();
        serde_json::from_str::<PinData>("{}").unwrap();
        serde_json::from_str::<PinsList>("{}").unwrap();
    }
}
