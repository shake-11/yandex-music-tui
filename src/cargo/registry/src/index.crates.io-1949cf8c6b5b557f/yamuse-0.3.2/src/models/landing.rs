//! the landing page: blocks, charts and the entities they hold.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Id;
use super::album::Album;
use super::playlist::{GeneratedPlaylist, Playlist};
use super::track::{Track, TrackId};

/// a promotional banner shown on the landing page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Promotion {
    /// promotion id.
    pub promo_id: Option<String>,
    /// title.
    pub title: Option<String>,
    /// subtitle.
    pub subtitle: Option<String>,
    /// heading.
    pub heading: Option<String>,
    /// web url.
    pub url: Option<String>,
    /// deep-link url scheme.
    pub url_scheme: Option<String>,
    /// text color.
    pub text_color: Option<String>,
    /// background gradient.
    pub gradient: Option<String>,
    /// image uri template.
    pub image: Option<String>,
}

/// a track reference plus the time it was added, inside a play context.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackShortOld {
    /// reference to the track.
    pub track_id: Option<TrackId>,
    /// when the track was added.
    pub timestamp: Option<String>,
}

/// a "continue listening" context — where playback last happened.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlayContext {
    /// context kind, for example `playlist`, `album` or `radio`.
    pub context: Option<String>,
    /// identifier of the context item.
    pub context_item: Option<String>,
    /// tracks recently played in this context.
    pub tracks: Vec<TrackShortOld>,
    /// context payload, still free-form upstream.
    pub payload: Option<Value>,
}

/// a link to a themed mix — a colored tile on the landing page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MixLink {
    /// title.
    pub title: Option<String>,
    /// web url.
    pub url: Option<String>,
    /// deep-link url scheme.
    pub url_scheme: Option<String>,
    /// text color.
    pub text_color: Option<String>,
    /// background color.
    pub background_color: Option<String>,
    /// background image uri template.
    pub background_image_uri: Option<String>,
    /// white cover variant.
    pub cover_white: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
}

/// a track's standing within a chart.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Chart {
    /// current position.
    pub position: Option<i64>,
    /// movement description.
    pub progress: Option<String>,
    /// listener count.
    pub listeners: Option<i64>,
    /// position shift since the previous chart.
    pub shift: Option<i64>,
    /// background color.
    pub bg_color: Option<String>,
    /// reference to the track.
    pub track_id: Option<TrackId>,
}

/// a track together with its chart standing.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChartItem {
    /// the track.
    pub track: Option<Track>,
    /// its chart standing.
    pub chart: Option<Chart>,
}

/// a chart page, backed by a playlist of ranked tracks.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChartInfo {
    /// chart id.
    pub id: Option<String>,
    /// chart type.
    pub r#type: Option<String>,
    /// origin tag used in `from` parameters.
    pub type_for_from: Option<String>,
    /// chart title.
    pub title: Option<String>,
    /// chart menu, still free-form upstream.
    pub menu: Option<Value>,
    /// the playlist backing the chart.
    pub chart: Option<Playlist>,
    /// chart description.
    pub chart_description: Option<String>,
}

/// the payload of a [`BlockEntity`], resolved according to its type.
///
/// the api tags each entity with a `type` string and puts a differently-shaped
/// object in `data`. rather than guess, [`BlockEntity::data`] resolves the pair
/// into this enum on demand — an unrecognized type keeps its raw json instead of
/// being dropped.
///
/// the catalogue variants are boxed for the same reason as [`SearchEntity`](super::SearchEntity):
/// an inline `Album` or `Playlist` makes the enum kilobytes wide.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BlockEntityData {
    /// `personal-playlist`.
    PersonalPlaylist(Box<GeneratedPlaylist>),
    /// `promotion`.
    Promotion(Box<Promotion>),
    /// `album`.
    Album(Box<Album>),
    /// `playlist`.
    Playlist(Box<Playlist>),
    /// `chart-item`.
    ChartItem(Box<ChartItem>),
    /// `play-context`.
    PlayContext(Box<PlayContext>),
    /// `mix-link`.
    MixLink(Box<MixLink>),
    /// a type this crate does not model yet; the payload is preserved verbatim.
    Unknown(Value),
}

/// one item inside a landing block: an album, playlist, promotion, ...
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BlockEntity {
    /// entity id.
    pub id: Option<String>,
    /// entity type, which drives the shape of `data`.
    pub r#type: Option<String>,
    /// the raw entity payload; call [`data`](Self::data) for the typed form.
    pub data: Option<Value>,
    /// entity description.
    pub description: Option<String>,
    /// formatted (html) entity description.
    pub description_formatted: Option<String>,
    /// last-updated timestamp, as iso 8601.
    pub last_updated: Option<String>,
}

impl BlockEntity {
    /// resolve the payload into its typed form according to `type`.
    ///
    /// returns `None` only when the entity carries no payload at all. a payload
    /// whose type is unrecognized, or which fails to match its expected shape,
    /// comes back as [`BlockEntityData::Unknown`] rather than an error — a landing
    /// page must never fail to load over one unfamiliar tile.
    #[must_use]
    pub fn data(&self) -> Option<BlockEntityData> {
        let raw = self.data.clone()?;
        let typed = |value: Value| -> Option<BlockEntityData> {
            Some(match self.r#type.as_deref()? {
                "personal-playlist" => {
                    BlockEntityData::PersonalPlaylist(Box::new(serde_json::from_value(value).ok()?))
                },
                "promotion" => {
                    BlockEntityData::Promotion(Box::new(serde_json::from_value(value).ok()?))
                },
                "album" => BlockEntityData::Album(Box::new(serde_json::from_value(value).ok()?)),
                "playlist" => {
                    BlockEntityData::Playlist(Box::new(serde_json::from_value(value).ok()?))
                },
                "chart-item" => {
                    BlockEntityData::ChartItem(Box::new(serde_json::from_value(value).ok()?))
                },
                "play-context" => {
                    BlockEntityData::PlayContext(Box::new(serde_json::from_value(value).ok()?))
                },
                "mix-link" => {
                    BlockEntityData::MixLink(Box::new(serde_json::from_value(value).ok()?))
                },
                _ => return None,
            })
        };
        Some(typed(raw.clone()).unwrap_or(BlockEntityData::Unknown(raw)))
    }
}

/// a block of a landing page: a titled row of entities.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Block {
    /// block id.
    pub id: Option<String>,
    /// block type.
    pub r#type: Option<String>,
    /// origin tag used in `from` parameters.
    pub type_for_from: Option<String>,
    /// block title.
    pub title: Option<String>,
    /// items in the block.
    pub entities: Vec<BlockEntity>,
    /// block description.
    pub description: Option<String>,
    /// block-level data, still free-form upstream.
    pub data: Option<Value>,
    /// background image uri template.
    pub background_image_url: Option<String>,
    /// background video identifier.
    pub background_video_id: Option<String>,
    /// background video url.
    pub background_video_url: Option<String>,
    /// play-context reference driving the block's playback, still free-form.
    pub play_context: Option<Value>,
}

/// a landing page: an ordered list of blocks.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Landing {
    /// whether the halloween ("pumpkin") theme is active.
    pub pumpkin: Option<bool>,
    /// content id of the landing.
    pub content_id: Option<Id>,
    /// the page blocks.
    pub blocks: Vec<Block>,
}

/// a landing list of ids: new releases, new playlists or podcasts.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LandingList {
    /// list type.
    pub r#type: Option<String>,
    /// origin tag used in `from` parameters.
    pub type_for_from: Option<String>,
    /// list title.
    pub title: Option<String>,
    /// list id.
    pub id: Option<String>,
    /// new release album ids.
    pub new_releases: Vec<i64>,
    /// new playlist references, still free-form upstream.
    pub new_playlists: Vec<Value>,
    /// podcast album ids.
    pub podcasts: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(json: &str) -> BlockEntity {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn entity_payloads_resolve_by_their_type_tag() {
        let album = entity(r#"{"type":"album","data":{"id":1,"title":"A"}}"#);
        assert!(
            matches!(album.data(), Some(BlockEntityData::Album(a)) if a.title.as_deref() == Some("A"))
        );

        let promo = entity(r#"{"type":"promotion","data":{"promoId":"p1"}}"#);
        assert!(matches!(promo.data(), Some(BlockEntityData::Promotion(_))));

        let personal = entity(r#"{"type":"personal-playlist","data":{"type":"playlistOfTheDay"}}"#);
        assert!(matches!(
            personal.data(),
            Some(BlockEntityData::PersonalPlaylist(_))
        ));

        let playlist = entity(r#"{"type":"playlist","data":{"kind":1}}"#);
        assert!(matches!(
            playlist.data(),
            Some(BlockEntityData::Playlist(_))
        ));

        let chart = entity(r#"{"type":"chart-item","data":{"chart":{"position":1}}}"#);
        assert!(matches!(chart.data(), Some(BlockEntityData::ChartItem(_))));

        let context = entity(r#"{"type":"play-context","data":{"context":"playlist"}}"#);
        assert!(matches!(
            context.data(),
            Some(BlockEntityData::PlayContext(_))
        ));

        let mix = entity(r#"{"type":"mix-link","data":{"title":"Mix"}}"#);
        assert!(matches!(mix.data(), Some(BlockEntityData::MixLink(_))));
    }

    #[test]
    fn an_unknown_type_keeps_its_payload_instead_of_dropping_it() {
        let unknown = entity(r#"{"type":"brand-new-block","data":{"anything":[1,2]}}"#);
        match unknown.data() {
            Some(BlockEntityData::Unknown(value)) => {
                assert_eq!(value, serde_json::json!({"anything": [1, 2]}));
            },
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn a_payload_that_does_not_match_its_type_degrades_to_unknown() {
        // upstream reshapes a block: better an Unknown tile than a failed page
        let mismatched = entity(r#"{"type":"album","data":"not an object"}"#);
        assert!(matches!(
            mismatched.data(),
            Some(BlockEntityData::Unknown(_))
        ));
    }

    #[test]
    fn an_entity_without_a_payload_resolves_to_nothing() {
        assert!(entity(r#"{"type":"album"}"#).data().is_none());
        assert!(entity("{}").data().is_none());
    }

    #[test]
    fn an_untagged_payload_is_preserved_as_unknown() {
        let untagged = entity(r#"{"data":{"x":1}}"#);
        assert!(matches!(untagged.data(), Some(BlockEntityData::Unknown(_))));
    }

    #[test]
    fn a_landing_page_deserializes_with_its_blocks() {
        let landing: Landing = serde_json::from_str(
            r#"{"pumpkin":false,"contentId":"main","blocks":[{"id":"b1","entities":[{"type":"album","data":{"id":1}}]}]}"#,
        )
        .unwrap();
        assert_eq!(landing.blocks.len(), 1);
        assert_eq!(landing.blocks[0].entities.len(), 1);
        assert_eq!(landing.content_id.unwrap().to_string(), "main");
    }

    #[test]
    fn landing_models_survive_an_empty_object() {
        serde_json::from_str::<Landing>("{}").unwrap();
        serde_json::from_str::<Block>("{}").unwrap();
        serde_json::from_str::<BlockEntity>("{}").unwrap();
        serde_json::from_str::<LandingList>("{}").unwrap();
        serde_json::from_str::<ChartInfo>("{}").unwrap();
        serde_json::from_str::<Chart>("{}").unwrap();
        serde_json::from_str::<ChartItem>("{}").unwrap();
        serde_json::from_str::<Promotion>("{}").unwrap();
        serde_json::from_str::<PlayContext>("{}").unwrap();
        serde_json::from_str::<MixLink>("{}").unwrap();
    }
}
