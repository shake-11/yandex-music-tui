//! the personalised feed.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::album::Album;
use super::artist::Artist;
use super::playlist::GeneratedPlaylist;
use super::track::Track;

/// a track bundled with its ad parameters, as served inside a [`Day`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackWithAds {
    /// ad slot type.
    pub r#type: Option<String>,
    /// the track.
    pub track: Option<Track>,
}

/// an album with the tracks surfaced for it inside a feed [`Event`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AlbumEvent {
    /// the album.
    pub album: Option<Album>,
    /// tracks surfaced from the album.
    pub tracks: Vec<Track>,
}

/// an artist with surfaced tracks and similar artists inside a feed [`Event`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistEvent {
    /// the artist.
    pub artist: Option<Artist>,
    /// tracks surfaced from the artist.
    pub tracks: Vec<Track>,
    /// similar artists derived from listening history.
    pub similar_to_artists_from_history: Vec<Artist>,
}

/// a single event in a [`Day`]: a release, a recommendation, a promotion, ...
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Event {
    /// event id.
    pub id: Option<String>,
    /// event type.
    pub r#type: Option<String>,
    /// origin tag used in `from` parameters.
    pub type_for_from: Option<String>,
    /// event title.
    pub title: Option<String>,
    /// tracks featured in the event.
    pub tracks: Vec<Track>,
    /// artist sub-events featured in the event.
    pub artists: Vec<ArtistEvent>,
    /// album sub-events featured in the event.
    pub albums: Vec<AlbumEvent>,
    /// free-form message blocks.
    pub message: Option<Value>,
    /// device descriptor, still free-form upstream.
    pub device: Option<Value>,
    /// number of tracks in the event.
    pub tracks_count: Option<i64>,
    /// associated genre.
    pub genre: Option<String>,
    /// background descriptor, for promotion events.
    pub background: Option<Value>,
    /// promotion category.
    pub category: Option<String>,
    /// event description.
    pub description: Option<String>,
    /// promotion heading.
    pub heading: Option<String>,
    /// image position hint.
    pub image_position: Option<String>,
    /// promotion id.
    pub promo_id: Option<String>,
    /// promotion type.
    pub promotion_type: Option<String>,
    /// promotion start date.
    pub start_date: Option<String>,
    /// event subtitle.
    pub subtitle: Option<String>,
    /// subtitle link url.
    pub subtitle_url: Option<String>,
    /// event tags, still free-form upstream.
    pub tags: Option<Value>,
    /// title link url.
    pub title_url: Option<String>,
}

/// a single day of the [`Feed`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Day {
    /// date, as `yyyy-mm-dd`.
    pub day: Option<String>,
    /// events of the day.
    pub events: Vec<Event>,
    /// tracks to play, with ad slots.
    pub tracks_to_play_with_ads: Vec<TrackWithAds>,
    /// tracks to play.
    pub tracks_to_play: Vec<Track>,
}

/// the personalised feed.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Feed {
    /// whether more events can be loaded.
    pub can_get_more_events: Option<bool>,
    /// whether the halloween ("pumpkin") theme is active.
    pub pumpkin: Option<bool>,
    /// whether the onboarding wizard has been completed.
    pub is_wizard_passed: Option<bool>,
    /// personalised generated playlists.
    pub generated_playlists: Vec<GeneratedPlaylist>,
    /// headline blocks, still free-form upstream.
    pub headlines: Vec<Value>,
    /// today's date, as `yyyy-mm-dd`.
    pub today: Option<String>,
    /// the feed days.
    pub days: Vec<Day>,
    /// revision token for the next page, still free-form upstream.
    pub next_revision: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_feed_nests_days_events_and_sub_events() {
        let feed: Feed = serde_json::from_str(
            r#"{
                "today":"2026-07-30","canGetMoreEvents":true,
                "generatedPlaylists":[{"type":"playlistOfTheDay","ready":true}],
                "days":[{"day":"2026-07-30","events":[{
                    "id":"e1","type":"artist","title":"New from A",
                    "artists":[{"artist":{"id":1},"tracks":[{"id":10}],"similarToArtistsFromHistory":[{"id":2}]}],
                    "albums":[{"album":{"id":100},"tracks":[{"id":11}]}]
                }],"tracksToPlay":[{"id":10}]}]
            }"#,
        )
        .unwrap();

        assert_eq!(feed.today.as_deref(), Some("2026-07-30"));
        assert_eq!(feed.generated_playlists.len(), 1);
        let event = &feed.days[0].events[0];
        assert_eq!(event.artists[0].similar_to_artists_from_history.len(), 1);
        assert_eq!(event.albums[0].tracks.len(), 1);
        assert_eq!(feed.days[0].tracks_to_play.len(), 1);
    }

    #[test]
    fn feed_models_survive_an_empty_object() {
        serde_json::from_str::<Feed>("{}").unwrap();
        serde_json::from_str::<Day>("{}").unwrap();
        serde_json::from_str::<Event>("{}").unwrap();
        serde_json::from_str::<AlbumEvent>("{}").unwrap();
        serde_json::from_str::<ArtistEvent>("{}").unwrap();
        serde_json::from_str::<TrackWithAds>("{}").unwrap();
    }
}
