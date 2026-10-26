//! the track model, its nested value objects and its download variants.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::album::Album;
use super::artist::Artist;
use super::landing::Chart;
use super::{ContentRestrictions, CoverDerivedColors, Id};
use crate::error::{Error, Result};
use crate::transport::Transport;

/// salt used to sign direct download links.
const SIGN_SALT: &str = "XGRlBW9FXlekgbPrRHuSiA";

/// a content provider or label major associated with a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Major {
    /// major identifier.
    pub id: Option<i64>,
    /// major name.
    pub name: Option<String>,
}

/// loudness normalization parameters, replaygain-style.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Normalization {
    /// gain adjustment in db.
    pub gain: Option<f64>,
    /// peak sample value.
    pub peak: Option<f64>,
}

/// a matched range of a track's lyrics, used by the "poetry lover" feature.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PoetryLoverMatch {
    /// start character offset within the matched line.
    pub begin: Option<i64>,
    /// end character offset within the matched line.
    pub end: Option<i64>,
    /// line number of the match.
    pub line: Option<i64>,
}

/// availability of lyrics for a track.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LyricsInfo {
    /// whether time-synced (lrc) lyrics are available.
    pub has_available_sync_lyrics: Option<bool>,
    /// whether plain-text lyrics are available.
    pub has_available_text_lyrics: Option<bool>,
}

/// fade-in and fade-out timing for smart previews and crossfade.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Fade {
    /// fade-in start time in seconds.
    pub in_start: Option<f64>,
    /// fade-in stop time in seconds.
    pub in_stop: Option<f64>,
    /// fade-out start time in seconds.
    pub out_start: Option<f64>,
    /// fade-out stop time in seconds.
    pub out_stop: Option<f64>,
}

/// parameters describing the auto-selected preview of a track.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SmartPreviewParams {
    /// preview duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// fade timing for the preview.
    pub fade: Option<Fade>,
}

/// track metadata as embedded in file tags.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MetaData {
    /// album title.
    pub album: Option<String>,
    /// volume (disc) number.
    pub volume: Option<i64>,
    /// release year.
    pub year: Option<i64>,
    /// track number within the volume.
    pub number: Option<i64>,
    /// genre.
    pub genre: Option<String>,
    /// lyricist credit.
    pub lyricist: Option<String>,
    /// version, for example "remastered".
    pub version: Option<String>,
    /// composer credit.
    pub composer: Option<String>,
}

/// ebu r128 loudness measurements.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct R128 {
    /// integrated loudness, in lufs.
    pub i: Option<f64>,
    /// true peak, in dbtp.
    pub tp: Option<f64>,
}

/// the label or major that owns a track's lyrics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LyricsMajor {
    /// identifier.
    pub id: Option<i64>,
    /// internal name.
    pub name: Option<String>,
    /// display name.
    pub pretty_name: Option<String>,
}

/// a music track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Track {
    /// unique track identifier.
    pub id: Option<Id>,
    /// track title.
    pub title: Option<String>,
    /// whether the track is available for playback.
    pub available: Option<bool>,
    /// performing artists.
    pub artists: Vec<Artist>,
    /// albums the track belongs to.
    pub albums: Vec<Album>,
    /// whether available for premium users.
    pub available_for_premium_users: Option<bool>,
    /// whether lyrics are available.
    pub lyrics_available: Option<bool>,
    /// matched lyric ranges from the poetry-lover feature.
    pub poetry_lover_matches: Vec<PoetryLoverMatch>,
    /// whether this is the "best" track.
    pub best: Option<bool>,
    /// canonical track id when this track is a duplicate.
    pub real_id: Option<Id>,
    /// open graph image uri template.
    pub og_image: Option<String>,
    /// track kind, for example `music` or `podcast-episode`.
    pub r#type: Option<String>,
    /// cover uri template.
    pub cover_uri: Option<String>,
    /// content provider major.
    pub major: Option<Major>,
    /// duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// storage directory.
    pub storage_dir: Option<String>,
    /// file size in bytes.
    pub file_size: Option<i64>,
    /// track substituted for this one when unavailable.
    pub substituted: Option<Box<Track>>,
    /// track this one matched against, in search results.
    pub matched_track: Option<Box<Track>>,
    /// loudness normalization parameters.
    pub normalization: Option<Normalization>,
    /// error code when the track could not be resolved.
    pub error: Option<String>,
    /// whether the track can be published.
    pub can_publish: Option<bool>,
    /// publication state.
    pub state: Option<String>,
    /// desired visibility.
    pub desired_visibility: Option<String>,
    /// original filename, for user-uploaded tracks.
    pub filename: Option<String>,
    /// embedded tag metadata.
    pub meta_data: Option<MetaData>,
    /// regions where available.
    pub regions: Vec<String>,
    /// whether available as a ringback tone.
    pub available_as_rbt: Option<bool>,
    /// content warning marker.
    pub content_warning: Option<String>,
    /// explicit marker.
    pub explicit: Option<bool>,
    /// preview duration in milliseconds.
    pub preview_duration_ms: Option<i64>,
    /// whether the full track is available without a subscription.
    pub available_full_without_permission: Option<bool>,
    /// version, for example "remastered".
    pub version: Option<String>,
    /// whether playback position is remembered, as for podcasts.
    pub remember_position: Option<bool>,
    /// background video uri for the track.
    pub background_video_uri: Option<String>,
    /// short description, for podcasts.
    pub short_description: Option<String>,
    /// whether suitable for children.
    pub is_suitable_for_children: Option<bool>,
    /// source of the track.
    pub track_source: Option<String>,
    /// options the track is available for.
    pub available_for_options: Vec<String>,
    /// lyrics availability flags.
    pub lyrics_info: Option<LyricsInfo>,
    /// sharing flag.
    pub track_sharing_flag: Option<String>,
    /// colors derived from the cover.
    pub derived_colors: Option<CoverDerivedColors>,
    /// crossfade timings.
    pub fade: Option<Fade>,
    /// smart preview parameters.
    pub smart_preview_params: Option<SmartPreviewParams>,
    /// special audio resource identifiers.
    pub special_audio_resources: Vec<String>,
    /// disclaimers.
    pub disclaimers: Vec<String>,
    /// background video identifier.
    pub background_video_id: Option<String>,
    /// player identifier.
    pub player_id: Option<String>,
    /// ebu r128 loudness measurements.
    pub r128: Option<R128>,
    /// podcast episode kind, for podcast episodes only.
    pub podcast_episode_type: Option<String>,
    /// publication date of a podcast episode, as iso 8601.
    pub pub_date: Option<String>,
    /// chart standing, when the track came from a chart context.
    pub chart: Option<Chart>,
    /// content availability restrictions.
    pub content_restrictions: Option<ContentRestrictions>,
}

impl Track {
    /// the numeric track id, dropping any `:album` suffix.
    ///
    /// # errors
    ///
    /// returns [`Error::IdMissing`] when the track carries no usable id.
    pub fn numeric_id(&self) -> Result<i64> {
        self.id
            .as_ref()
            .and_then(Id::as_number)
            .ok_or(Error::IdMissing("track id"))
    }

    /// the performing artists joined for display, for example `"a, b"`.
    #[must_use]
    pub fn artists_name(&self) -> String {
        self.artists
            .iter()
            .filter_map(|a| a.name.as_deref())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// the cover url at a concrete size, for example `400x400`.
    #[must_use]
    pub fn cover_url(&self, size: &str) -> Option<String> {
        self.cover_uri
            .as_deref()
            .map(|uri| super::normalize_image_url(uri, size))
    }

    /// the album this track is primarily attributed to.
    #[must_use]
    pub fn album(&self) -> Option<&Album> {
        self.albums.first()
    }
}

/// a compact reference to a track inside a playlist.
///
/// carries the ids needed to fetch the full [`Track`]; the `track` field is only
/// populated by endpoints that inline the whole object.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackShort {
    /// track id.
    pub id: Option<Id>,
    /// timestamp the track was added at.
    pub timestamp: Option<String>,
    /// album id the track is referenced from.
    pub album_id: Option<String>,
    /// play counter.
    pub play_count: Option<i64>,
    /// whether recently added.
    pub recent: Option<bool>,
    /// chart placement, still free-form upstream.
    pub chart: Option<Value>,
    /// the full track, when inlined by the endpoint.
    pub track: Option<Track>,
    /// original index within the source list.
    pub original_index: Option<i64>,
    /// original index within the shuffled source list.
    pub original_shuffle_index: Option<i64>,
}

/// a bare track and album id pair, used by landing blocks and queues.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackId {
    /// id of the reference.
    pub id: Option<i64>,
    /// track id.
    pub track_id: Option<i64>,
    /// album id.
    pub album_id: Option<i64>,
}

/// one download variant of a track: a codec, a bitrate and a way to resolve a
/// short-lived direct file url.
///
/// this is the legacy lossy path (mp3 and aac). for flac use
/// [`LosslessDownloadInfo`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DownloadInfo {
    /// audio codec, for example `mp3` or `aac`.
    pub codec: Option<String>,
    /// bitrate in kbit/s.
    pub bitrate_in_kbps: Option<i64>,
    /// whether gain is applied.
    pub gain: Option<bool>,
    /// whether this is a preview rather than the full track.
    pub preview: Option<bool>,
    /// url of the xml document describing how to build the direct link.
    pub download_info_url: Option<String>,
    /// whether the link is already direct.
    pub direct: Option<bool>,
}

impl DownloadInfo {
    /// resolve the short-lived direct download url.
    ///
    /// the api answers with a small xml document whose parts are hashed into the
    /// final path. the resolved url is valid for roughly a minute — request it
    /// immediately before downloading, or the cdn answers 410.
    ///
    /// # errors
    ///
    /// returns [`Error::IdMissing`] when there is no info url, or a transport
    /// error when the xml cannot be fetched or is malformed.
    pub async fn direct_link(&self, transport: &Transport) -> Result<String> {
        let info_url = self
            .download_info_url
            .as_deref()
            .ok_or(Error::IdMissing("downloadInfoUrl"))?;
        let xml = transport.retrieve_text(info_url).await?;
        build_direct_link(&xml)
    }
}

/// build the direct download url from the download-info xml.
fn build_direct_link(xml: &str) -> Result<String> {
    let tag = |name: &str| -> Option<&str> {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let start = xml.find(&open)? + open.len();
        let end = xml[start..].find(&close)? + start;
        Some(&xml[start..end])
    };

    let (Some(host), Some(path), Some(ts), Some(s)) =
        (tag("host"), tag("path"), tag("ts"), tag("s"))
    else {
        return Err(Error::InvalidResponse(
            "download-info xml is missing host/path/ts/s".into(),
        ));
    };

    // the api's own scheme: md5 over the salt, the path without its leading
    // slash, and the `s` nonce.
    let mut hasher = Md5::new();
    hasher.update(SIGN_SALT.as_bytes());
    hasher.update(path.strip_prefix('/').unwrap_or(path).as_bytes());
    hasher.update(s.as_bytes());
    let sign = hex::encode(hasher.finalize());

    Ok(format!("https://{host}/get-mp3/{sign}/{ts}{path}"))
}

/// lossless (and other modern) download info from `/get-file-info`.
///
/// with `transport: encraw` the urls serve an aes-ctr-encrypted stream and
/// [`key`](Self::key) holds the hex decryption key. the endpoint falls back to a
/// lossy codec when a track has no lossless source — check
/// [`is_lossless`](Self::is_lossless).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LosslessDownloadInfo {
    /// track id this info belongs to.
    pub track_id: Option<String>,
    /// requested or granted quality, for example `lossless`.
    pub quality: Option<String>,
    /// actual codec served: `flac`, `flac-mp4`, `aac`, `mp3`, ...
    pub codec: Option<String>,
    /// bitrate in kbit/s.
    pub bitrate: Option<i64>,
    /// transport, typically `encraw` for an aes-ctr-encrypted raw stream.
    pub transport: Option<String>,
    /// hex aes-ctr key used to decrypt an `encraw` stream, when encrypted.
    pub key: Option<String>,
    /// candidate stream urls; any one works.
    pub urls: Vec<String>,
    /// single stream url, from the older response shape.
    pub url: Option<String>,
    /// file size in bytes, when known.
    pub size: Option<i64>,
    /// whether loudness-normalization gain data is available.
    pub gain: Option<bool>,
    /// canonical track id when the requested id is an alias.
    pub real_id: Option<String>,
}

impl LosslessDownloadInfo {
    /// whether this is a true lossless (flac) variant.
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        matches!(self.codec.as_deref(), Some("flac" | "flac-mp4"))
    }

    /// whether the stream is aes-ctr encrypted and needs the key applied.
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        self.key.is_some()
    }

    /// every candidate stream url, deduplicated, in preference order.
    #[must_use]
    pub fn links(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for url in self.urls.iter().chain(self.url.as_ref()) {
            if !url.is_empty() && !seen.contains(url) {
                seen.push(url.clone());
            }
        }
        seen
    }
}

/// lyrics of a track.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackLyrics {
    /// url the raw lyrics text is fetched from.
    pub download_url: Option<String>,
    /// lyric identifier.
    pub lyric_id: Option<i64>,
    /// external lyric identifier.
    pub external_lyric_id: Option<String>,
    /// writer credits.
    pub writers: Vec<String>,
    /// lyrics provider major.
    pub major: Option<LyricsMajor>,
}

impl TrackLyrics {
    /// fetch the raw lyrics text, plain or lrc to match the requested format.
    ///
    /// the lyrics endpoint returns a url rather than the text itself, so reading
    /// the words takes this second call.
    ///
    /// # errors
    ///
    /// returns [`Error::IdMissing`] when there is no download url, or a transport
    /// error when the text cannot be fetched.
    pub async fn text(&self, transport: &Transport) -> Result<String> {
        let url = self
            .download_url
            .as_deref()
            .ok_or(Error::IdMissing("lyrics downloadUrl"))?;
        transport.retrieve_text(url).await
    }
}

/// a track together with tracks similar to it.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SimilarTracks {
    /// the reference track.
    pub track: Option<Track>,
    /// tracks similar to the reference.
    pub similar_tracks: Vec<Track>,
}

/// full information about a track, including related content.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackFullInfo {
    /// the track itself.
    pub track: Option<Track>,
    /// similar tracks.
    pub similar_tracks: Vec<Track>,
    /// other albums the track appears in.
    pub also_in_albums: Vec<Track>,
    /// title aliases.
    pub aliases: Vec<String>,
    /// related artists.
    pub artists: Vec<Artist>,
    /// other versions of the track, keyed by version label such as `live`.
    pub other_versions: std::collections::HashMap<String, Vec<Track>>,
}

/// a track trailer.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackTrailer {
    /// trailer title.
    pub title: Option<String>,
    /// the trailer track.
    pub track: Option<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_direct_link_is_built_from_the_info_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><download-info>
            <host>storage.mds.yandex.net</host>
            <path>/get-mp3/abc/1.mp3</path>
            <ts>0005f2</ts>
            <region>225</region>
            <s>deadbeef</s>
        </download-info>"#;

        let link = build_direct_link(xml).unwrap();

        // md5(salt + path-without-leading-slash + s), verified independently
        let mut hasher = Md5::new();
        hasher.update(SIGN_SALT.as_bytes());
        hasher.update(b"get-mp3/abc/1.mp3");
        hasher.update(b"deadbeef");
        let expected = hex::encode(hasher.finalize());

        assert_eq!(
            link,
            format!("https://storage.mds.yandex.net/get-mp3/{expected}/0005f2/get-mp3/abc/1.mp3")
        );
    }

    #[test]
    fn an_incomplete_xml_is_rejected() {
        assert!(build_direct_link("<download-info><host>h</host></download-info>").is_err());
        assert!(build_direct_link("not xml").is_err());
        assert!(build_direct_link("").is_err());
    }

    #[test]
    fn lossless_detection_covers_both_flac_codecs() {
        let flac = LosslessDownloadInfo {
            codec: Some("flac".into()),
            ..Default::default()
        };
        let flac_mp4 = LosslessDownloadInfo {
            codec: Some("flac-mp4".into()),
            ..Default::default()
        };
        let aac = LosslessDownloadInfo {
            codec: Some("aac".into()),
            ..Default::default()
        };
        assert!(flac.is_lossless());
        assert!(flac_mp4.is_lossless());
        assert!(!aac.is_lossless());
        assert!(!LosslessDownloadInfo::default().is_lossless());
    }

    #[test]
    fn links_merge_both_url_shapes_and_deduplicate() {
        let info = LosslessDownloadInfo {
            urls: vec!["https://a".into(), "https://b".into(), "https://a".into()],
            url: Some("https://b".into()),
            ..Default::default()
        };
        assert_eq!(
            info.links(),
            vec!["https://a".to_string(), "https://b".to_string()]
        );

        let only_single = LosslessDownloadInfo {
            url: Some("https://c".into()),
            ..Default::default()
        };
        assert_eq!(only_single.links(), vec!["https://c".to_string()]);

        assert!(LosslessDownloadInfo::default().links().is_empty());
    }

    #[test]
    fn encryption_is_flagged_by_the_presence_of_a_key() {
        assert!(!LosslessDownloadInfo::default().is_encrypted());
        let encrypted = LosslessDownloadInfo {
            key: Some("ab".into()),
            ..Default::default()
        };
        assert!(encrypted.is_encrypted());
    }

    #[test]
    fn tracks_accept_both_id_shapes_and_normalize_them() {
        let numeric: Track = serde_json::from_str(r#"{"id":42}"#).unwrap();
        let compound: Track = serde_json::from_str(r#"{"id":"42:1001"}"#).unwrap();
        assert_eq!(numeric.numeric_id().unwrap(), 42);
        assert_eq!(compound.numeric_id().unwrap(), 42);
        assert!(Track::default().numeric_id().is_err());
    }

    #[test]
    fn artist_names_join_for_display() {
        let track: Track =
            serde_json::from_str(r#"{"artists":[{"name":"A"},{"name":"B"},{}]}"#).unwrap();
        assert_eq!(track.artists_name(), "A, B");
        assert_eq!(Track::default().artists_name(), "");
    }

    #[test]
    fn cover_urls_resolve_the_size_placeholder() {
        let track: Track =
            serde_json::from_str(r#"{"coverUri":"avatars.yandex.net/get-music/1/%%"}"#).unwrap();
        assert_eq!(
            track.cover_url("200x200").as_deref(),
            Some("https://avatars.yandex.net/get-music/1/200x200")
        );
        assert!(Track::default().cover_url("200x200").is_none());
    }

    #[test]
    fn a_substituted_track_nests_without_blowing_the_stack() {
        let track: Track =
            serde_json::from_str(r#"{"id":1,"substituted":{"id":2,"title":"other"}}"#).unwrap();
        let substituted = track.substituted.as_ref().unwrap();
        assert_eq!(substituted.title.as_deref(), Some("other"));
    }

    #[test]
    fn other_versions_deserialize_as_a_keyed_map() {
        let info: TrackFullInfo = serde_json::from_str(
            r#"{"track":{"id":1},"otherVersions":{"live":[{"id":2}],"remix":[{"id":3},{"id":4}]}}"#,
        )
        .unwrap();
        assert_eq!(info.other_versions["live"].len(), 1);
        assert_eq!(info.other_versions["remix"].len(), 2);
    }

    #[test]
    fn track_models_survive_an_empty_object() {
        serde_json::from_str::<Track>("{}").unwrap();
        serde_json::from_str::<TrackShort>("{}").unwrap();
        serde_json::from_str::<TrackId>("{}").unwrap();
        serde_json::from_str::<DownloadInfo>("{}").unwrap();
        serde_json::from_str::<LosslessDownloadInfo>("{}").unwrap();
        serde_json::from_str::<TrackLyrics>("{}").unwrap();
        serde_json::from_str::<SimilarTracks>("{}").unwrap();
        serde_json::from_str::<TrackFullInfo>("{}").unwrap();
        serde_json::from_str::<TrackTrailer>("{}").unwrap();
    }
}
