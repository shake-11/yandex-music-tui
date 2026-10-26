//! track endpoints.

use serde_json::Value;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{
    DownloadInfo, LosslessDownloadInfo, Mutation, ShotEvent, SimilarTracks, Supplement, Track,
    TrackFullInfo, TrackLyrics, TrackTrailer,
};
use crate::params::{Params, segment};
use crate::sign::{
    FILE_INFO_CODECS, FILE_INFO_TRANSPORT, sign_file_info, sign_track, track_id_to_number,
};
use crate::types::{BatchObject, LyricsFormat};

/// a playback progress report for [`Client::play_audio`].
///
/// only `track_id`, `album_id` and `from` are required; the rest default to
/// sensible zeros and "now".
#[derive(Debug, Clone, Default)]
pub struct PlayAudioReport {
    /// track id.
    pub track_id: i64,
    /// album id the track belongs to.
    pub album_id: i64,
    /// name of the client the track is played from.
    pub from: String,
    /// playlist id, when a playlist is playing.
    pub playlist_id: Option<String>,
    /// whether the track is played from cache.
    pub from_cache: bool,
    /// unique play id.
    pub play_id: Option<String>,
    /// user id. defaults to the authenticated account.
    pub uid: Option<i64>,
    /// iso timestamp of the event. defaults to now.
    pub timestamp: Option<String>,
    /// track length in seconds.
    pub track_length_seconds: u64,
    /// seconds played in total.
    pub total_played_seconds: u64,
    /// final played-seconds position.
    pub end_position_seconds: u64,
    /// iso client timestamp. defaults to now.
    pub client_now: Option<String>,
}

impl PlayAudioReport {
    /// start a report for a track played from `from`.
    #[must_use]
    pub fn new(track_id: i64, album_id: i64, from: impl Into<String>) -> Self {
        Self {
            track_id,
            album_id,
            from: from.into(),
            ..Self::default()
        }
    }
}

impl Client {
    /// fetch one or many tracks by id.
    ///
    /// `POST /tracks` — the ids travel as a repeated form field, which is why a
    /// lookup is a `POST`.
    pub async fn tracks(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> Result<Vec<Track>> {
        self.tracks_with_positions(track_ids, true).await
    }

    /// [`tracks`](Self::tracks), choosing whether to include album positions.
    pub async fn tracks_with_positions(
        &self,
        track_ids: impl IntoIterator<Item = impl std::fmt::Display>,
        with_positions: bool,
    ) -> Result<Vec<Track>> {
        let extra = Params::new().add("with-positions", with_positions);
        self.get_batch(BatchObject::Track, track_ids, extra).await
    }

    /// fetch one track by id, or `None` when it does not resolve.
    pub async fn track(&self, track_id: impl std::fmt::Display) -> Result<Option<Track>> {
        Ok(self.tracks([track_id]).await?.into_iter().next())
    }

    /// fetch the available lossy download variants for a track.
    ///
    /// `GET /tracks/{id}/download-info`
    ///
    /// each variant still needs [`DownloadInfo::direct_link`] to resolve an actual
    /// url. for flac use [`track_lossless_info`](Self::track_lossless_info).
    pub async fn track_download_info(
        &self,
        track_id: impl std::fmt::Display,
    ) -> Result<Vec<DownloadInfo>> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/download-info"), Params::new())
            .await
    }

    /// fetch lossless (flac) download info for a track.
    ///
    /// `GET /get-file-info` — signed, and the `codecs`/`transport` query values
    /// must match the ones the signature was computed over.
    ///
    /// the stream comes back aes-ctr encrypted; see
    /// [`LosslessDownloadInfo::key`] and the `download` feature.
    ///
    /// # errors
    ///
    /// returns [`Error::IdMissing`] when `track_id` has no numeric form.
    pub async fn track_lossless_info(
        &self,
        track_id: impl std::fmt::Display,
        quality: &str,
    ) -> Result<LosslessDownloadInfo> {
        let id = track_id_to_number(&track_id.to_string())
            .ok_or(Error::IdMissing("numeric track id"))?;
        let sign = sign_file_info(id, quality);
        let params = Params::new()
            .add("ts", sign.ts)
            .add("trackId", id)
            .add("quality", quality)
            .add("codecs", FILE_INFO_CODECS)
            .add("transports", FILE_INFO_TRANSPORT)
            .add("sign", sign.value);

        let value = self.get_value("/get-file-info", params).await?;
        let info = value.get("downloadInfo").cloned().unwrap_or(Value::Null);
        self.decode(info)
    }

    /// fetch the lyrics of a track.
    ///
    /// `GET /tracks/{id}/lyrics` — signed, and authorization is required.
    ///
    /// the response carries a url rather than the words; call
    /// [`TrackLyrics::text`] for those.
    ///
    /// # errors
    ///
    /// returns [`Error::NotFound`] when the track has no lyrics, or
    /// [`Error::IdMissing`] when `track_id` has no numeric form.
    pub async fn track_lyrics(
        &self,
        track_id: impl std::fmt::Display,
        format: LyricsFormat,
    ) -> Result<TrackLyrics> {
        let display = track_id.to_string();
        let id = track_id_to_number(&display).ok_or(Error::IdMissing("numeric track id"))?;
        let sign = sign_track(id);
        let params = Params::new()
            .add("format", format)
            .add("timeStamp", sign.timestamp)
            .add("sign", sign.value);
        self.get(&format!("/tracks/{id}/lyrics"), params).await
    }

    /// fetch tracks similar to a track.
    ///
    /// `GET /tracks/{id}/similar`
    pub async fn track_similar(&self, track_id: impl std::fmt::Display) -> Result<SimilarTracks> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/similar"), Params::new())
            .await
    }

    /// fetch full information about a track.
    ///
    /// `GET /tracks/{id}/full-info`
    pub async fn track_full_info(&self, track_id: impl std::fmt::Display) -> Result<TrackFullInfo> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/full-info"), Params::new())
            .await
    }

    /// fetch the trailer of a track.
    ///
    /// `GET /tracks/{id}/trailer`
    pub async fn track_trailer(&self, track_id: impl std::fmt::Display) -> Result<TrackTrailer> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/trailer"), Params::new())
            .await
    }

    /// fetch supplementary information about a track: legacy lyrics and videos.
    ///
    /// `GET /tracks/{id}/supplement`
    pub async fn track_supplement(&self, track_id: impl std::fmt::Display) -> Result<Supplement> {
        let track_id = segment(track_id);
        self.get(&format!("/tracks/{track_id}/supplement"), Params::new())
            .await
    }

    /// report the current playback state of a track.
    ///
    /// `POST /play-audio`
    pub async fn play_audio(&self, report: &PlayAudioReport) -> Result<Mutation> {
        let now = crate::time::iso_now;
        let uid = report.uid.or_else(|| self.account_uid());
        let form = Params::new()
            .add("track-id", report.track_id)
            .add("from-cache", report.from_cache)
            .add("from", &report.from)
            .add("play-id", report.play_id.as_deref().unwrap_or(""))
            .add("uid", uid.map(|u| u.to_string()).unwrap_or_default())
            .add("timestamp", report.timestamp.clone().unwrap_or_else(now))
            .add("track-length-seconds", report.track_length_seconds)
            .add("total-played-seconds", report.total_played_seconds)
            .add("end-position-seconds", report.end_position_seconds)
            .add("album-id", report.album_id)
            .add("playlist-id", report.playlist_id.as_deref().unwrap_or(""))
            .add("client-now", report.client_now.clone().unwrap_or_else(now));

        let value = self.post_form_value("/play-audio", form).await?;
        Client::acknowledged(&value)
    }

    /// fetch an ad or an alice shot to play after a track.
    ///
    /// `GET /after-track`
    pub async fn after_track(
        &self,
        next_track_id: impl std::fmt::Display,
        context_item: &str,
        prev_track_id: Option<i64>,
    ) -> Result<ShotEvent> {
        let params = Params::new()
            .add("from", "mobile-landing-origin-default")
            .maybe("prevTrackId", prev_track_id)
            .add("nextTrackId", next_track_id)
            .add("context", "playlist")
            .add("contextItem", context_item)
            .add("types", "shot");

        let value = self.get_value("/after-track", params).await?;
        let event = value.get("shotEvent").cloned().unwrap_or(Value::Null);
        self.decode(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_play_report_starts_with_only_its_required_fields_set() {
        let report = PlayAudioReport::new(42, 1001, "desktop_win");
        assert_eq!(report.track_id, 42);
        assert_eq!(report.album_id, 1001);
        assert_eq!(report.from, "desktop_win");
        assert!(!report.from_cache);
        assert!(report.play_id.is_none());
        assert_eq!(report.total_played_seconds, 0);
    }
}
