//! typed models for every api payload.
//!
//! every struct derives `Deserialize` with `#[serde(default)]`, so a field the api
//! stops sending becomes `None` instead of failing the whole response. that
//! matters against a private api that adds and drops fields without notice: a
//! model here degrades rather than breaks.
//!
//! fields the api returns as free-form or still-unmodelled objects are kept as
//! [`serde_json::Value`] rather than dropped, so nothing is ever silently lost.

use std::fmt;

use serde::{Deserialize, Serialize};

pub mod account;
pub mod album;
pub mod artist;
pub mod clip;
pub mod concert;
pub mod credit;
pub mod device_auth;
pub mod disclaimer;
pub mod feed;
pub mod genre;
pub mod label;
pub mod landing;
pub mod like;
pub mod metatag;
pub mod misc;
pub mod music_history;
pub mod pin;
pub mod playlist;
pub mod presaves;
pub mod queue;
pub mod rotor;
pub mod search;
pub mod shot;
pub mod supplement;
pub mod track;
pub mod user;
pub mod video;

pub use account::*;
pub use album::*;
pub use artist::*;
pub use clip::*;
pub use concert::*;
pub use credit::*;
pub use device_auth::*;
pub use disclaimer::*;
pub use feed::*;
pub use genre::*;
pub use label::*;
pub use landing::*;
pub use like::*;
pub use metatag::*;
pub use misc::*;
pub use music_history::*;
pub use pin::*;
pub use playlist::*;
pub use presaves::*;
pub use queue::*;
pub use rotor::*;
pub use search::*;
pub use shot::*;
pub use supplement::*;
pub use track::*;
pub use user::*;
pub use video::*;

/// an identifier the api sends as either a json number or a json string.
///
/// track and clip ids are numeric in most responses and quoted in others (and a
/// track id may even be compound, `"42:1001"`). rather than guess, both shapes
/// deserialize here and [`Display`](fmt::Display) renders whichever arrived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    /// the api sent a json number.
    Number(i64),
    /// the api sent a json string, possibly compound (`"trackId:albumId"`).
    Text(String),
}

impl Id {
    /// the numeric value, parsing a string form and dropping any `:album` suffix.
    #[must_use]
    pub fn as_number(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(s) => crate::sign::track_id_to_number(s),
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Text(s) => f.write_str(s),
        }
    }
}

impl From<i64> for Id {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

/// a field the api answers as either a boolean flag or the list it is a flag for.
///
/// `hasOptions` is the case that forced this: the name says `boolean`, and the
/// live api sends the options themselves. rather than pick one and be wrong half
/// the time, both shapes deserialize here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagOrList {
    /// the api sent a plain flag.
    Flag(bool),
    /// the api sent the list, which being non-empty is itself the flag.
    List(Vec<serde_json::Value>),
}

impl FlagOrList {
    /// whether the flag is set — the boolean itself, or a non-empty list.
    #[must_use]
    pub fn is_set(&self) -> bool {
        match self {
            Self::Flag(flag) => *flag,
            Self::List(items) => !items.is_empty(),
        }
    }

    /// the list, when the api sent one.
    #[must_use]
    pub fn list(&self) -> &[serde_json::Value] {
        match self {
            Self::Flag(_) => &[],
            Self::List(items) => items,
        }
    }
}

/// colors derived from a cover image, used to theme players and cards.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CoverDerivedColors {
    /// average color of the cover.
    pub average: Option<String>,
    /// color suited for text drawn over the wave animation.
    pub wave_text: Option<String>,
    /// background color for the mini player.
    pub mini_player: Option<String>,
    /// accent color.
    pub accent: Option<String>,
}

/// cover art descriptor.
///
/// a cover references either a single `uri` template or a list of `items_uri`
/// tiles (mosaic covers, typically playlists). replace the `%%` placeholder in a
/// uri with a size such as `400x400` to get a concrete image url.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Cover {
    /// cover kind, for example `mosaic` or `pic`.
    pub r#type: Option<String>,
    /// uri template of a single image.
    pub uri: Option<String>,
    /// uri templates of the tiles composing a mosaic cover.
    pub items_uri: Vec<String>,
    /// storage directory of the cover.
    pub dir: Option<String>,
    /// version tag of the cover.
    pub version: Option<String>,
    /// whether the cover is custom (legacy flag).
    pub custom: Option<bool>,
    /// whether the cover is custom.
    pub is_custom: Option<bool>,
    /// copyright holder name.
    pub copyright_name: Option<String>,
    /// copyright c-line.
    pub copyright_cline: Option<String>,
    /// uri prefix.
    pub prefix: Option<String>,
    /// error code when the cover could not be resolved.
    pub error: Option<String>,
    /// dominant color.
    pub color: Option<String>,
    /// colors derived from the cover image.
    pub derived_colors: Option<CoverDerivedColors>,
    /// url of an animated or video cover.
    pub video_url: Option<String>,
}

impl Cover {
    /// resolve the cover uri at a concrete size, for example `400x400`.
    ///
    /// returns `None` when the cover carries no single-image uri (mosaic covers
    /// expose [`items_uri`](Self::items_uri) instead).
    ///
    /// # examples
    ///
    /// ```
    /// # use yamuse::models::Cover;
    /// let cover: Cover = serde_json::from_str(r#"{"uri":"avatars.yandex.net/get-music/1/%%"}"#)?;
    /// assert_eq!(
    ///     cover.url("400x400").as_deref(),
    ///     Some("https://avatars.yandex.net/get-music/1/400x400"),
    /// );
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    #[must_use]
    pub fn url(&self, size: &str) -> Option<String> {
        self.uri
            .as_deref()
            .map(|uri| normalize_image_url(uri, size))
    }
}

/// turn a `%%`-templated image uri into an absolute https url at `size`.
///
/// the api is inconsistent about the scheme: some payloads carry a bare host,
/// some a protocol-relative `//host`, some a full url. all three land here.
#[must_use]
pub fn normalize_image_url(uri: &str, size: &str) -> String {
    let sized = uri.replace("%%", size);
    if sized.starts_with("//") {
        format!("https:{sized}")
    } else if sized.starts_with("http://") || sized.starts_with("https://") {
        sized
    } else {
        format!("https://{sized}")
    }
}

/// a square icon with a background color, used by genres and stations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Icon {
    /// background color in hex.
    pub background_color: Option<String>,
    /// uri template of the icon image.
    pub image_url: Option<String>,
}

/// an external link attached to an artist or label.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Link {
    /// display title of the link.
    pub title: Option<String>,
    /// target url (artist and album external links).
    pub href: Option<String>,
    /// target url (the key some link endpoints use instead of `href`).
    pub url: Option<String>,
    /// image url accompanying the link.
    pub img_url: Option<String>,
    /// secondary line of text.
    pub subtitle: Option<String>,
    /// link kind, for example `official` or `social`.
    pub r#type: Option<String>,
    /// social network name, when `type` is `social`.
    pub social_network: Option<String>,
}

impl Link {
    /// the target url, from whichever key the endpoint used.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.href.as_deref().or(self.url.as_deref())
    }
}

/// content availability restrictions and disclaimers for an entity.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentRestrictions {
    /// whether the content is available.
    pub available: Option<bool>,
    /// human-readable disclaimers.
    pub disclaimers: Vec<String>,
}

/// a page action on an entity page, for example a donation button.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExtraAction {
    /// action type, for example `donation`.
    pub r#type: Option<String>,
    /// button label.
    pub title: Option<String>,
    /// accent color in hex.
    pub color: Option<String>,
    /// target url.
    pub url: Option<String>,
}

/// pagination cursor describing one page of a larger result set.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Pager {
    /// total number of items across all pages.
    pub total: Option<u32>,
    /// current page index, zero-based.
    pub page: Option<u32>,
    /// number of items per page.
    pub per_page: Option<u32>,
}

impl Pager {
    /// whether another page follows this one.
    #[must_use]
    pub fn has_next(&self) -> bool {
        match (self.total, self.page, self.per_page) {
            (Some(total), Some(page), Some(per_page)) if per_page > 0 => {
                (page + 1) * per_page < total
            },
            _ => false,
        }
    }

    /// whether another page follows the one numbered `expected`.
    ///
    /// the walking helpers count pages themselves rather than trusting the `page`
    /// the api echoes back: an endpoint that always reports page 0 would otherwise
    /// make [`has_next`](Self::has_next) true forever and the walk never end.
    pub(crate) fn has_next_after(&self, expected: u32) -> bool {
        match (self.total, self.per_page) {
            (Some(total), Some(per_page)) if per_page > 0 => (expected + 1) * per_page < total,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_accept_both_json_shapes() {
        assert_eq!(serde_json::from_str::<Id>("42").unwrap(), Id::Number(42));
        assert_eq!(
            serde_json::from_str::<Id>(r#""42""#).unwrap(),
            Id::Text("42".into())
        );
        assert_eq!(
            serde_json::from_str::<Id>(r#""42:1001""#).unwrap(),
            Id::Text("42:1001".into())
        );
    }

    #[test]
    fn ids_render_and_normalize() {
        assert_eq!(Id::Number(42).to_string(), "42");
        assert_eq!(Id::Text("42:1001".into()).to_string(), "42:1001");
        assert_eq!(Id::Number(42).as_number(), Some(42));
        assert_eq!(Id::Text("42:1001".into()).as_number(), Some(42));
        assert_eq!(Id::Text("nope".into()).as_number(), None);
    }

    #[test]
    fn image_urls_normalize_every_scheme_the_api_sends() {
        assert_eq!(
            normalize_image_url("avatars.yandex.net/get-music/%%", "400x400"),
            "https://avatars.yandex.net/get-music/400x400"
        );
        assert_eq!(
            normalize_image_url("//avatars.yandex.net/%%", "50x50"),
            "https://avatars.yandex.net/50x50"
        );
        assert_eq!(
            normalize_image_url("https://avatars.yandex.net/%%", "1000x1000"),
            "https://avatars.yandex.net/1000x1000"
        );
        assert_eq!(
            normalize_image_url("http://host/%%", "1x1"),
            "http://host/1x1"
        );
    }

    #[test]
    fn a_mosaic_cover_has_no_single_url() {
        let cover: Cover =
            serde_json::from_str(r#"{"type":"mosaic","itemsUri":["a/%%","b/%%"]}"#).unwrap();
        assert!(cover.url("100x100").is_none());
        assert_eq!(cover.items_uri.len(), 2);
    }

    #[test]
    fn links_report_whichever_url_key_arrived() {
        let with_href: Link = serde_json::from_str(r#"{"href":"https://a"}"#).unwrap();
        let with_url: Link = serde_json::from_str(r#"{"url":"https://b"}"#).unwrap();
        let neither: Link = serde_json::from_str("{}").unwrap();
        assert_eq!(with_href.target(), Some("https://a"));
        assert_eq!(with_url.target(), Some("https://b"));
        assert_eq!(neither.target(), None);
    }

    #[test]
    fn pager_detects_a_following_page() {
        let mid = Pager {
            total: Some(100),
            page: Some(0),
            per_page: Some(20),
        };
        let last = Pager {
            total: Some(100),
            page: Some(4),
            per_page: Some(20),
        };
        let exact = Pager {
            total: Some(40),
            page: Some(1),
            per_page: Some(20),
        };
        assert!(mid.has_next());
        assert!(!last.has_next());
        assert!(!exact.has_next());
        assert!(!Pager::default().has_next());
        // a zero page size would divide by nothing useful — treated as "no next"
        assert!(
            !Pager {
                total: Some(10),
                page: Some(0),
                per_page: Some(0)
            }
            .has_next()
        );
    }

    #[test]
    fn models_tolerate_an_empty_object() {
        // the api routinely omits whole blocks; every model must survive that
        serde_json::from_str::<Cover>("{}").unwrap();
        serde_json::from_str::<Link>("{}").unwrap();
        serde_json::from_str::<Icon>("{}").unwrap();
        serde_json::from_str::<Pager>("{}").unwrap();
        serde_json::from_str::<ContentRestrictions>("{}").unwrap();
        serde_json::from_str::<ExtraAction>("{}").unwrap();
        serde_json::from_str::<CoverDerivedColors>("{}").unwrap();
    }

    #[test]
    fn unknown_fields_are_ignored_rather_than_fatal() {
        // upstream adds fields without notice; that must not break deserialization
        let cover: Cover =
            serde_json::from_str(r#"{"uri":"a/%%","brandNewFieldFromUpstream":123}"#).unwrap();
        assert_eq!(cover.uri.as_deref(), Some("a/%%"));
    }
}
