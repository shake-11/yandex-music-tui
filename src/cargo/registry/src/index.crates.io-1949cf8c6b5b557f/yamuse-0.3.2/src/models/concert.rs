//! concerts: events, feeds, locations and page scaffolds.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Cover;

/// the minimum ticket price for a concert.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertMinPrice {
    /// price value.
    pub value: Option<f64>,
    /// currency code.
    pub currency: Option<String>,
    /// currency symbol.
    pub currency_symbol: Option<String>,
}

/// cashback offered for a concert.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertCashback {
    /// cashback title.
    pub title: Option<String>,
    /// cashback percentage.
    pub value_percent: Option<f64>,
}

/// event-type information for a concert.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertEventInfo {
    /// event type; `concert` and `festival` are the known values.
    pub r#type: Option<String>,
}

/// a free-text concert description with a source.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertDescription {
    /// description text.
    pub text: Option<String>,
    /// source of the description.
    pub source: Option<String>,
}

/// a concert event.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Concert {
    /// concert id.
    pub id: Option<String>,
    /// image uri templates.
    pub images: Vec<String>,
    /// single image uri.
    pub image_url: Option<String>,
    /// concert title.
    pub concert_title: Option<String>,
    /// afisha (poster service) url.
    pub afisha_url: Option<String>,
    /// city.
    pub city: Option<String>,
    /// venue.
    pub place: Option<String>,
    /// address.
    pub address: Option<String>,
    /// date and time.
    pub datetime: Option<String>,
    /// content rating.
    pub content_rating: Option<String>,
    /// minimum ticket price.
    pub min_price: Option<ConcertMinPrice>,
    /// cashback offer.
    pub cashback: Option<ConcertCashback>,
    /// event-type info.
    pub event_info: Option<ConcertEventInfo>,
    /// cover art.
    pub cover: Option<Cover>,
    /// data session id, from the alternative response shape.
    pub data_session_id: Option<String>,
}

/// an artist's upcoming concerts.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistConcerts {
    /// artist title.
    pub artist_title: Option<String>,
    /// the concerts.
    pub concerts: Vec<Concert>,
}

/// detailed information about a single concert.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertInfo {
    /// the concert.
    pub concert: Option<Concert>,
    /// minimum ticket price.
    pub min_price: Option<ConcertMinPrice>,
    /// cover art.
    pub covers: Vec<Cover>,
    /// concert description.
    pub description: Option<ConcertDescription>,
    /// id of the lead artist.
    pub lead_artist_id: Option<i64>,
}

/// the payload of a [`ConcertFeedItem`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertFeedItemData {
    /// the concert.
    pub concert: Option<Concert>,
    /// minimum ticket price.
    pub min_price: Option<ConcertMinPrice>,
}

/// an item in a concert feed.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertFeedItem {
    /// item type; `concert_item` is the known value.
    pub r#type: Option<String>,
    /// item payload.
    pub data: Option<ConcertFeedItemData>,
}

/// a feed of concerts.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertFeed {
    /// the feed items.
    pub items: Vec<ConcertFeedItem>,
}

/// a concert location, that is, a city.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertLocation {
    /// location id.
    pub id: Option<i64>,
    /// location name.
    pub name: Option<String>,
}

/// the available concert locations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertLocations {
    /// the locations.
    pub locations: Vec<ConcertLocation>,
}

/// a pagination range inside a concert tab config.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertTabRange {
    /// offset from the start.
    pub offset: Option<u32>,
    /// maximum number of items.
    pub limit: Option<u32>,
}

/// the ranges configuring the top and feed sections of the concert tab.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertTabConfigData {
    /// "top" section range.
    pub top: Option<ConcertTabRange>,
    /// "feed" section range.
    pub feed: Option<ConcertTabRange>,
}

/// the concert tab configuration.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertTabConfig {
    /// the configuration data.
    pub config: Option<ConcertTabConfigData>,
}

/// the skeleton — lazy-render scaffold — of a concert page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConcertSkeleton {
    /// skeleton id.
    pub id: Option<String>,
    /// skeleton title.
    pub title: Option<String>,
    /// skeleton blocks, still free-form upstream.
    pub blocks: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_concert_deserializes_with_its_price_and_event_info() {
        let concert: Concert = serde_json::from_str(
            r#"{"id":"c1","concertTitle":"Live","city":"Moscow","datetime":"2026-09-01T20:00:00",
                "minPrice":{"value":2500.0,"currency":"RUB","currencySymbol":"₽"},
                "eventInfo":{"type":"festival"},
                "cashback":{"valuePercent":10.0}}"#,
        )
        .unwrap();
        assert_eq!(concert.city.as_deref(), Some("Moscow"));
        assert_eq!(concert.min_price.unwrap().value, Some(2500.0));
        assert_eq!(
            concert.event_info.unwrap().r#type.as_deref(),
            Some("festival")
        );
        assert_eq!(concert.cashback.unwrap().value_percent, Some(10.0));
    }

    #[test]
    fn a_concert_feed_nests_its_items() {
        let feed: ConcertFeed = serde_json::from_str(
            r#"{"items":[{"type":"concert_item","data":{"concert":{"id":"c1"}}}]}"#,
        )
        .unwrap();
        assert_eq!(
            feed.items[0]
                .data
                .as_ref()
                .unwrap()
                .concert
                .as_ref()
                .unwrap()
                .id
                .as_deref(),
            Some("c1")
        );
    }

    #[test]
    fn concert_models_survive_an_empty_object() {
        serde_json::from_str::<Concert>("{}").unwrap();
        serde_json::from_str::<ArtistConcerts>("{}").unwrap();
        serde_json::from_str::<ConcertInfo>("{}").unwrap();
        serde_json::from_str::<ConcertFeed>("{}").unwrap();
        serde_json::from_str::<ConcertLocations>("{}").unwrap();
        serde_json::from_str::<ConcertTabConfig>("{}").unwrap();
        serde_json::from_str::<ConcertSkeleton>("{}").unwrap();
        serde_json::from_str::<ConcertMinPrice>("{}").unwrap();
        serde_json::from_str::<ConcertCashback>("{}").unwrap();
        serde_json::from_str::<ConcertDescription>("{}").unwrap();
        serde_json::from_str::<ConcertLocation>("{}").unwrap();
    }
}
