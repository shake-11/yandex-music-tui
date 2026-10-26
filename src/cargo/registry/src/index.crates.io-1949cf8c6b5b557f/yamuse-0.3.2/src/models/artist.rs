//! the artist model and the many blocks an artist page is assembled from.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::album::Album;
use super::clip::Clip;
use super::landing::Chart;
use super::misc::TrailerInfo;
use super::playlist::{CustomWave, Playlist, PlaylistId};
use super::track::Track;
use super::video::Video;
use super::{ContentRestrictions, Cover, CoverDerivedColors, ExtraAction, Id, Link, Pager};

/// aggregate counts of an artist's content.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Counts {
    /// number of tracks.
    pub tracks: Option<i64>,
    /// number of albums where the artist is the primary author.
    pub direct_albums: Option<i64>,
    /// number of albums the artist also appears on.
    pub also_albums: Option<i64>,
    /// number of tracks the artist also appears on.
    pub also_tracks: Option<i64>,
}

/// popularity ratings of an artist over time windows.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ratings {
    /// monthly rank.
    pub month: Option<i64>,
    /// weekly rank.
    pub week: Option<i64>,
    /// daily rank.
    pub day: Option<i64>,
}

/// a textual description with an optional source uri.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Description {
    /// description text.
    pub text: Option<String>,
    /// source uri of the description.
    pub uri: Option<String>,
}

/// an artist's recent listening statistics.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Stats {
    /// number of listeners over the last month.
    pub last_month_listeners: Option<i64>,
    /// change in monthly listeners since the previous period.
    pub last_month_listeners_delta: Option<i64>,
}

/// a vinyl record offered for an artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Vinyl {
    /// store url of the offer.
    pub url: Option<String>,
    /// record title.
    pub title: Option<String>,
    /// release year.
    pub year: Option<i64>,
    /// price in the store's currency.
    pub price: Option<f64>,
    /// media format.
    pub media: Option<String>,
    /// offer identifier.
    pub offer_id: Option<i64>,
    /// ids of the artists featured on the record.
    pub artist_ids: Vec<i64>,
    /// cover picture uri template.
    pub picture: Option<String>,
}

/// a music artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Artist {
    /// unique artist identifier.
    ///
    /// numeric on most endpoints and a string on `likes/artists`, so it is an
    /// [`Id`] rather than either one — call [`Id::as_number`] for the value.
    pub id: Option<Id>,
    /// error code, present when the artist could not be resolved.
    pub error: Option<String>,
    /// reason the artist is unavailable.
    pub reason: Option<String>,
    /// artist name.
    pub name: Option<String>,
    /// artist cover art.
    pub cover: Option<Cover>,
    /// whether this represents "various artists".
    pub various: Option<bool>,
    /// whether the artist is a composer.
    pub composer: Option<bool>,
    /// genres associated with the artist.
    pub genres: Vec<String>,
    /// open graph image uri template.
    pub og_image: Option<String>,
    /// alternative image uri template.
    pub op_image: Option<String>,
    /// aggregate content counts.
    pub counts: Option<Counts>,
    /// whether the artist is available.
    pub available: Option<bool>,
    /// popularity ratings.
    pub ratings: Option<Ratings>,
    /// external links: official site, socials.
    pub links: Vec<Link>,
    /// whether concert tickets are available.
    pub tickets_available: Option<bool>,
    /// number of likes.
    pub likes_count: Option<i64>,
    /// a selection of the artist's popular tracks.
    pub popular_tracks: Vec<Track>,
    /// regions where the artist is available.
    pub regions: Vec<String>,
    /// hand-written description.
    pub hand_made_description: Option<String>,
    /// structured description.
    pub description: Option<Description>,
    /// countries associated with the artist.
    pub countries: Vec<String>,
    /// english wikipedia link.
    pub en_wikipedia_link: Option<String>,
    /// database aliases.
    pub db_aliases: Vec<String>,
    /// career start date.
    pub init_date: Option<String>,
    /// career end date.
    pub end_date: Option<String>,
    /// yoomoney identifier, for donations.
    pub ya_money_id: Option<String>,
    /// disclaimers.
    pub disclaimers: Vec<String>,
    /// content availability restrictions.
    pub content_restrictions: Option<ContentRestrictions>,
    /// cut-out (transparent) cover art.
    pub cutout_cover: Option<Cover>,
    /// colors derived from the cover image.
    pub derived_colors: Option<CoverDerivedColors>,
    /// whether the artist has a trailer.
    pub has_trailer: Option<bool>,
    /// whether to suppress search-sourced pictures.
    pub no_pictures_from_search: Option<bool>,
    /// like timestamp, as iso 8601, in a likes context.
    pub timestamp: Option<String>,
    /// artist trailer descriptor, still free-form upstream.
    pub trailer: Option<Value>,
    /// donation and support info, still free-form upstream.
    pub donation_info: Option<Value>,
    /// extra page actions, for example a donation button.
    pub extra_actions: Vec<ExtraAction>,
    /// decomposed name parts, still free-form upstream.
    pub decomposed: Option<Value>,
}

impl Artist {
    /// the cover url at a concrete size, for example `400x400`.
    #[must_use]
    pub fn cover_url(&self, size: &str) -> Option<String> {
        self.cover
            .as_ref()
            .and_then(|cover| cover.url(size))
            .or_else(|| {
                self.og_image
                    .as_deref()
                    .map(|uri| super::normalize_image_url(uri, size))
            })
    }
}

/// one page of tracks belonging to an artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistTracks {
    /// the tracks on this page.
    pub tracks: Vec<Track>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

/// one external link on an artist's page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistLink {
    /// link title.
    pub title: Option<String>,
    /// link subtitle.
    pub subtitle: Option<String>,
    /// target url.
    pub url: Option<String>,
    /// icon image url.
    pub img_url: Option<String>,
}

/// the collection of external links shown on an artist's page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistLinks {
    /// the artist's external links.
    pub links: Vec<ArtistLink>,
}

/// one page of an artist's albums.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistAlbums {
    /// the albums on this page.
    pub albums: Vec<Album>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

/// artists similar to a given artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistSimilar {
    /// the reference artist.
    pub artist: Option<Artist>,
    /// artists similar to the reference.
    pub similar_artists: Vec<Artist>,
}

/// an artist together with their trailer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistTrailer {
    /// the artist.
    pub artist: Option<Artist>,
    /// the trailer.
    pub trailer: Option<TrailerInfo>,
}

/// a rich aggregate of everything shown on an artist's main page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BriefInfo {
    /// the artist.
    pub artist: Option<Artist>,
    /// the artist's albums.
    pub albums: Vec<Album>,
    /// playlists featuring the artist.
    pub playlists: Vec<Playlist>,
    /// albums the artist also appears on.
    pub also_albums: Vec<Album>,
    /// ids of the most recent releases.
    pub last_release_ids: Vec<i64>,
    /// the most recent releases.
    pub last_releases: Vec<Album>,
    /// a selection of popular tracks.
    pub popular_tracks: Vec<Track>,
    /// similar artists.
    pub similar_artists: Vec<Artist>,
    /// all available covers.
    pub all_covers: Vec<Cover>,
    /// upcoming concerts, still free-form upstream.
    pub concerts: Option<Value>,
    /// music videos.
    pub videos: Vec<Video>,
    /// vinyl records on offer.
    pub vinyls: Vec<Vinyl>,
    /// whether the artist has active promotions.
    pub has_promotions: Option<bool>,
    /// references to playlists by the artist.
    pub playlist_ids: Vec<PlaylistId>,
    /// listening statistics.
    pub stats: Option<Stats>,
    /// the artist's tracks that currently chart.
    pub tracks_in_chart: Vec<Chart>,
    /// bandlink scanner (smartlink) url for the artist.
    pub bandlink_scanner_link: Option<String>,
    /// the artist's clips.
    pub clips: Vec<Clip>,
    /// extra page actions, for example a donation button.
    pub extra_actions: Vec<ExtraAction>,
    /// the artist's custom wave (radio) descriptor.
    pub custom_wave: Option<CustomWave>,
    /// whether the artist has a trailer.
    pub has_trailer: Option<bool>,
    /// external links.
    pub links: Vec<Link>,
}

/// a fundraising goal attached to an artist donation.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistDonationGoal {
    /// goal title.
    pub title: Option<String>,
}

/// the payload of an [`ArtistDonationItem`]: a tip link and its goal.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistDonationData {
    /// tip or donation url.
    pub tip_url: Option<String>,
    /// the artist the donation supports.
    pub artist: Option<Artist>,
    /// the fundraising goal.
    pub goal: Option<ArtistDonationGoal>,
}

/// one donation block on an artist's page.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistDonationItem {
    /// donation block type.
    pub r#type: Option<String>,
    /// donation payload.
    pub data: Option<ArtistDonationData>,
}

/// the artist-donation block.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistDonations {
    /// the donation items.
    pub donations: Vec<ArtistDonationItem>,
}

/// detailed artist information.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistInfo {
    /// the artist.
    pub artist: Option<Artist>,
    /// number of likes.
    pub likes_count: Option<i64>,
    /// listening statistics.
    pub stats: Option<Stats>,
    /// trailer availability, still free-form upstream.
    pub trailer: Option<Value>,
    /// donation info, still free-form upstream.
    pub donation: Option<Value>,
    /// all available covers.
    pub covers: Vec<Cover>,
    /// artist description.
    pub description: Option<String>,
    /// artist type, for example `artist` or `podcaster`.
    pub artist_type: Option<String>,
}

/// the "about" block of an artist.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistAbout {
    /// the artist.
    pub artist: Option<Artist>,
    /// donation blocks.
    pub donations: Vec<ArtistDonationItem>,
    /// listening statistics.
    pub stats: Option<Stats>,
    /// artist description.
    pub description: Option<String>,
    /// external links.
    pub links: Vec<Link>,
    /// all available covers.
    pub covers: Vec<Cover>,
    /// artist type.
    pub artist_type: Option<String>,
}

/// the payload of an [`ArtistClipItem`]: a clip plus its artists.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistClipData {
    /// the clip.
    pub clip: Option<Clip>,
    /// artists featured in the clip.
    pub artists: Vec<Artist>,
}

/// one item in the artist-clips block.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistClipItem {
    /// item type.
    pub r#type: Option<String>,
    /// item payload.
    pub data: Option<ArtistClipData>,
}

/// the artist-clips block.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistClips {
    /// the clip items.
    pub items: Vec<ArtistClipItem>,
    /// pagination metadata.
    pub pager: Option<Pager>,
}

/// an artist page skeleton — the lazy-render scaffold.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistSkeleton {
    /// skeleton id.
    pub id: Option<String>,
    /// skeleton title.
    pub title: Option<String>,
    /// page blocks, still free-form upstream.
    pub blocks: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_urls_prefer_the_structured_cover_then_fall_back_to_og_image() {
        let structured: Artist =
            serde_json::from_str(r#"{"cover":{"uri":"c/%%"},"ogImage":"og/%%"}"#).unwrap();
        assert_eq!(
            structured.cover_url("50x50").as_deref(),
            Some("https://c/50x50")
        );

        let og_only: Artist = serde_json::from_str(r#"{"ogImage":"og/%%"}"#).unwrap();
        assert_eq!(
            og_only.cover_url("50x50").as_deref(),
            Some("https://og/50x50")
        );

        assert!(Artist::default().cover_url("50x50").is_none());
    }

    #[test]
    fn a_full_brief_info_deserializes() {
        let brief: BriefInfo = serde_json::from_str(
            r#"{
                "artist":{"id":1,"name":"A"},
                "albums":[{"id":10}],
                "popularTracks":[{"id":100}],
                "similarArtists":[{"id":2}],
                "stats":{"lastMonthListeners":5000},
                "vinyls":[{"title":"LP","price":2500.0}],
                "concerts":{"anything":true}
            }"#,
        )
        .unwrap();
        assert_eq!(brief.artist.unwrap().name.as_deref(), Some("A"));
        assert_eq!(brief.albums.len(), 1);
        assert_eq!(brief.stats.unwrap().last_month_listeners, Some(5000));
        assert_eq!(brief.vinyls[0].price, Some(2500.0));
        assert!(brief.concerts.is_some());
    }

    #[test]
    fn artist_models_survive_an_empty_object() {
        serde_json::from_str::<Artist>("{}").unwrap();
        serde_json::from_str::<ArtistTracks>("{}").unwrap();
        serde_json::from_str::<ArtistAlbums>("{}").unwrap();
        serde_json::from_str::<ArtistLinks>("{}").unwrap();
        serde_json::from_str::<ArtistSimilar>("{}").unwrap();
        serde_json::from_str::<ArtistTrailer>("{}").unwrap();
        serde_json::from_str::<BriefInfo>("{}").unwrap();
        serde_json::from_str::<ArtistInfo>("{}").unwrap();
        serde_json::from_str::<ArtistAbout>("{}").unwrap();
        serde_json::from_str::<ArtistClips>("{}").unwrap();
        serde_json::from_str::<ArtistDonations>("{}").unwrap();
        serde_json::from_str::<ArtistSkeleton>("{}").unwrap();
        serde_json::from_str::<Stats>("{}").unwrap();
        serde_json::from_str::<Vinyl>("{}").unwrap();
    }
}
