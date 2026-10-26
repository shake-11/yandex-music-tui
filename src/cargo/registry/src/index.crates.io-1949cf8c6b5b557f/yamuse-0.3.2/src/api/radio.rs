//! rotor (radio) endpoints.

use crate::client::Client;
use crate::error::Result;
use crate::models::{Dashboard, Mutation, StationResult, StationTracksResult, Status};
use crate::params::{Params, segment};
use crate::types::{Diversity, MoodEnergy, RotorFeedback, StationLanguage, StationType};

/// one listening-feedback event for [`Client::rotor_feedback`].
///
/// the rotor personalises a station from these, so a client that plays radio is
/// expected to report `radio_started`, then a `track_started`/`track_finished` or
/// `skip` per track.
#[derive(Debug, Clone, Default)]
pub struct RotorFeedbackReport {
    /// id of the track the event refers to.
    pub track_id: Option<i64>,
    /// id of the track batch the event belongs to.
    pub batch_id: Option<String>,
    /// where playback started, for example `mobile-radio-user-123`.
    pub from: Option<String>,
    /// seconds of the track played before the event.
    pub total_played_seconds: Option<f64>,
    /// event time in seconds since the epoch. defaults to now.
    pub timestamp: Option<f64>,
}

impl Client {
    /// fetch the rotor account status.
    ///
    /// `GET /rotor/account/status`
    pub async fn rotor_account_status(&self) -> Result<Status> {
        self.get("/rotor/account/status", Params::new()).await
    }

    /// fetch the personalized station dashboard.
    ///
    /// `GET /rotor/stations/dashboard`
    pub async fn rotor_dashboard(&self) -> Result<Dashboard> {
        self.get("/rotor/stations/dashboard", Params::new()).await
    }

    /// fetch the full list of available stations.
    ///
    /// `GET /rotor/stations/list`
    pub async fn rotor_stations(&self) -> Result<Vec<StationResult>> {
        let params = Params::new().add("language", self.language());
        self.get("/rotor/stations/list", params).await
    }

    /// fetch information about a specific station.
    ///
    /// `GET /rotor/station/{station}/info`
    ///
    /// `station` is a `type:tag` pair, for example `genre:pop`.
    pub async fn rotor_station_info(&self, station: &str) -> Result<Vec<StationResult>> {
        let station = segment(station);
        self.get(&format!("/rotor/station/{station}/info"), Params::new())
            .await
    }

    /// fetch the next batch of tracks from a station.
    ///
    /// `GET /rotor/station/{station}/tracks`
    ///
    /// pass the previous batch's last track id as `queue` to keep the sequence
    /// coherent; the first call leaves it `None`.
    pub async fn rotor_station_tracks(
        &self,
        station: &str,
        queue: Option<i64>,
    ) -> Result<StationTracksResult> {
        // the api takes either `queue` or `settings2`, not both.
        let params = match queue {
            Some(queue) => Params::new().add("queue", queue),
            None => Params::new().add("settings2", true),
        };
        let station = segment(station);
        self.get(&format!("/rotor/station/{station}/tracks"), params)
            .await
    }

    /// send listening feedback for a station.
    ///
    /// `POST /rotor/station/{station}/feedback`
    pub async fn rotor_feedback(
        &self,
        station: &str,
        event: RotorFeedback,
        report: &RotorFeedbackReport,
    ) -> Result<Mutation> {
        // the batch id belongs in the query, not the form body
        let query = Params::new().maybe("batch-id", report.batch_id.as_deref());
        let station = segment(station);
        let path = format!("/rotor/station/{station}/feedback");

        let timestamp = report.timestamp.unwrap_or_else(crate::time::unix_secs_f64);

        let form = Params::new()
            .add("type", event)
            .add("timestamp", timestamp)
            .maybe("trackId", report.track_id)
            .maybe("from", report.from.as_deref())
            .maybe("totalPlayedSeconds", report.total_played_seconds);

        let value = self.post_form_value_with(&path, &query, form, &[]).await?;
        Client::acknowledged(&value)
    }

    /// report that a radio session started.
    pub async fn rotor_radio_started(&self, station: &str, from: &str) -> Result<Mutation> {
        let report = RotorFeedbackReport {
            from: Some(from.to_owned()),
            ..Default::default()
        };
        self.rotor_feedback(station, RotorFeedback::RadioStarted, &report)
            .await
    }

    /// report that a track started playing.
    pub async fn rotor_track_started(&self, station: &str, track_id: i64) -> Result<Mutation> {
        let report = RotorFeedbackReport {
            track_id: Some(track_id),
            ..Default::default()
        };
        self.rotor_feedback(station, RotorFeedback::TrackStarted, &report)
            .await
    }

    /// report that a track played to the end.
    pub async fn rotor_track_finished(
        &self,
        station: &str,
        track_id: i64,
        total_played_seconds: f64,
    ) -> Result<Mutation> {
        let report = RotorFeedbackReport {
            track_id: Some(track_id),
            total_played_seconds: Some(total_played_seconds),
            ..Default::default()
        };
        self.rotor_feedback(station, RotorFeedback::TrackFinished, &report)
            .await
    }

    /// report that a track was skipped.
    pub async fn rotor_skip(
        &self,
        station: &str,
        track_id: i64,
        total_played_seconds: f64,
    ) -> Result<Mutation> {
        let report = RotorFeedbackReport {
            track_id: Some(track_id),
            total_played_seconds: Some(total_played_seconds),
            ..Default::default()
        };
        self.rotor_feedback(station, RotorFeedback::Skip, &report)
            .await
    }

    /// change the settings of a station.
    ///
    /// `POST /rotor/station/{station}/settings3`
    pub async fn rotor_set_station_settings(
        &self,
        station: &str,
        mood_energy: MoodEnergy,
        diversity: Diversity,
        language: StationLanguage,
        station_type: StationType,
    ) -> Result<Mutation> {
        let form = Params::new()
            .add("moodEnergy", mood_energy)
            .add("diversity", diversity)
            .add("type", station_type)
            .add("language", language);
        let station = segment(station);
        let value = self
            .post_form_value(&format!("/rotor/station/{station}/settings3"), form)
            .await?;
        Client::acknowledged(&value)
    }
}
