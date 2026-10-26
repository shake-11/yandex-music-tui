//! rotor (radio) stations, their settings and their track batches.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::track::Track;
use super::{Icon, Id as ScalarId};

/// one possible value of an enumerated station restriction.
///
/// the api calls this a "value"; named for what it actually is here, and to
/// avoid shadowing [`serde_json::Value`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RestrictionValue {
    /// value key.
    pub value: Option<String>,
    /// human-readable name.
    pub name: Option<String>,
    /// image url, for rich restriction values such as moods.
    pub image_url: Option<String>,
    /// serialized seed used to start a wave from this value.
    pub serialized_seed: Option<String>,
    /// whether this value represents the "unspecified" option.
    pub unspecified: Option<bool>,
}

/// an enumerated station restriction: a fixed set of possible values.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Enum {
    /// restriction type, `enum`.
    pub r#type: Option<String>,
    /// restriction name.
    pub name: Option<String>,
    /// the possible values.
    pub possible_values: Vec<RestrictionValue>,
}

/// a discrete-scale station restriction: a min/max range.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DiscreteScale {
    /// restriction type, `discrete-scale`.
    pub r#type: Option<String>,
    /// restriction name.
    pub name: Option<String>,
    /// minimum value.
    pub min: Option<RestrictionValue>,
    /// maximum value.
    pub max: Option<RestrictionValue>,
}

/// the settings a station exposes for personalization.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Restrictions {
    /// language restriction.
    pub language: Option<Enum>,
    /// diversity restriction.
    pub diversity: Option<Enum>,
    /// mood restriction.
    pub mood: Option<DiscreteScale>,
    /// energy restriction.
    pub energy: Option<DiscreteScale>,
    /// combined mood/energy restriction.
    pub mood_energy: Option<Enum>,
}

/// advertisement parameters attached to a station result.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AdParams {
    /// partner id.
    pub partner_id: Option<ScalarId>,
    /// category id.
    pub category_id: Option<ScalarId>,
    /// page reference.
    pub page_ref: Option<String>,
    /// target reference.
    pub target_ref: Option<String>,
    /// extra parameters, for example `user:{id}`.
    pub other_params: Option<String>,
    /// ad volume.
    pub ad_volume: Option<i64>,
    /// genre id.
    pub genre_id: Option<String>,
    /// genre name.
    pub genre_name: Option<String>,
}

/// a typed identifier of a rotor station, `{type}:{tag}`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Id {
    /// station type, for example `genre` or `user`.
    pub r#type: Option<String>,
    /// station tag within the type.
    pub tag: Option<String>,
}

impl Id {
    /// the canonical `type:tag` station identifier, when both parts are present.
    #[must_use]
    pub fn as_string(&self) -> Option<String> {
        Some(format!(
            "{}:{}",
            self.r#type.as_deref()?,
            self.tag.as_deref()?
        ))
    }
}

/// a rotor station descriptor.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Station {
    /// station id.
    pub id: Option<Id>,
    /// station name.
    pub name: Option<String>,
    /// station icon.
    pub icon: Option<Icon>,
    /// mts-branded icon.
    pub mts_icon: Option<Icon>,
    /// geocell icon.
    pub geocell_icon: Option<Icon>,
    /// origin tag used in `from` parameters.
    pub id_for_from: Option<String>,
    /// playback restrictions.
    pub restrictions: Option<Restrictions>,
    /// newer, enum-typed playback restrictions.
    pub restrictions2: Option<Restrictions>,
    /// full image url.
    pub full_image_url: Option<String>,
    /// mts full image url.
    pub mts_full_image_url: Option<String>,
    /// parent station id.
    pub parent_id: Option<Id>,
}

/// the selected personalization settings of a station.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StationSettings {
    /// selected language filter, for example `any`.
    pub language: Option<String>,
    /// selected mood on the legacy numeric scale.
    pub mood: Option<i64>,
    /// selected energy on the legacy numeric scale.
    pub energy: Option<i64>,
    /// selected mood/energy preset, for example `all`.
    pub mood_energy: Option<String>,
    /// selected diversity, for example `default`.
    pub diversity: Option<String>,
}

/// a station together with its settings and ad parameters.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StationResult {
    /// the station.
    pub station: Option<Station>,
    /// station personalization settings.
    pub settings: Option<StationSettings>,
    /// newer station personalization settings payload.
    pub settings2: Option<StationSettings>,
    /// ad parameters.
    pub ad_params: Option<AdParams>,
    /// explanation text.
    pub explanation: Option<String>,
    /// prerolls, still free-form upstream.
    pub prerolls: Option<Value>,
    /// "rup" title.
    pub rup_title: Option<String>,
    /// "rup" description.
    pub rup_description: Option<String>,
    /// custom display name.
    pub custom_name: Option<String>,
}

/// the personalized list of rotor stations.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Dashboard {
    /// dashboard id.
    pub dashboard_id: Option<String>,
    /// the available stations.
    pub stations: Vec<StationResult>,
    /// whether the halloween ("pumpkin") theme is active.
    pub pumpkin: Option<bool>,
}

/// acoustic parameters of a track within a radio sequence.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TrackParameters {
    /// beats per minute.
    pub bpm: Option<f64>,
    /// cover-derived hue.
    pub hue: Option<f64>,
    /// energy level.
    pub energy: Option<f64>,
}

/// one track in a station's playback sequence.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Sequence {
    /// sequence item type, for example `track`.
    pub r#type: Option<String>,
    /// the track.
    pub track: Option<Track>,
    /// whether the user has liked the track.
    pub liked: Option<bool>,
    /// acoustic parameters of the track in this sequence.
    pub track_parameters: Option<TrackParameters>,
}

/// a batch of tracks streamed from a rotor station.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StationTracksResult {
    /// station id.
    pub id: Option<Id>,
    /// the track sequence.
    pub sequence: Vec<Sequence>,
    /// batch id, echoed back as `queue` when requesting the next batch.
    pub batch_id: Option<String>,
    /// whether the halloween ("pumpkin") theme is active.
    pub pumpkin: Option<bool>,
    /// radio session id, used for rotor feedback.
    pub radio_session_id: Option<String>,
}

impl StationTracksResult {
    /// the tracks of this batch, in sequence order.
    #[must_use]
    pub fn tracks(&self) -> Vec<&Track> {
        self.sequence
            .iter()
            .filter_map(|item| item.track.as_ref())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_ids_compose_from_type_and_tag() {
        let id = Id {
            r#type: Some("genre".into()),
            tag: Some("pop".into()),
        };
        assert_eq!(id.as_string().as_deref(), Some("genre:pop"));
        assert!(Id::default().as_string().is_none());
    }

    #[test]
    fn a_track_batch_exposes_its_tracks_in_order() {
        let batch: StationTracksResult = serde_json::from_str(
            r#"{"id":{"type":"user","tag":"onyourwave"},"batchId":"b1","sequence":[
                {"type":"track","track":{"id":1},"liked":false,"trackParameters":{"bpm":128.0}},
                {"type":"track","track":{"id":2}},
                {"type":"ad"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(batch.tracks().len(), 2, "the ad slot carries no track");
        assert_eq!(batch.batch_id.as_deref(), Some("b1"));
        assert_eq!(batch.sequence[0].track_parameters.unwrap().bpm, Some(128.0));
    }

    #[test]
    fn both_restriction_payloads_deserialize() {
        let result: StationResult = serde_json::from_str(
            r#"{"station":{"restrictions":{"mood":{"type":"discrete-scale","min":{"value":"1"},"max":{"value":"4"}}},
                 "restrictions2":{"moodEnergy":{"type":"enum","possibleValues":[{"value":"all","name":"All"}]}}},
                "settings2":{"moodEnergy":"all","diversity":"default"}}"#,
        )
        .unwrap();
        let station = result.station.unwrap();
        assert!(station.restrictions.unwrap().mood.is_some());
        let mood_energy = station.restrictions2.unwrap().mood_energy.unwrap();
        assert_eq!(mood_energy.possible_values[0].name.as_deref(), Some("All"));
        assert_eq!(
            result.settings2.unwrap().mood_energy.as_deref(),
            Some("all")
        );
    }

    #[test]
    fn ad_params_accept_both_id_shapes() {
        let params: AdParams =
            serde_json::from_str(r#"{"partnerId":123,"categoryId":"cat-7"}"#).unwrap();
        assert_eq!(params.partner_id.unwrap().to_string(), "123");
        assert_eq!(params.category_id.unwrap().to_string(), "cat-7");
    }

    #[test]
    fn rotor_models_survive_an_empty_object() {
        serde_json::from_str::<Dashboard>("{}").unwrap();
        serde_json::from_str::<StationResult>("{}").unwrap();
        serde_json::from_str::<StationTracksResult>("{}").unwrap();
        serde_json::from_str::<Station>("{}").unwrap();
        serde_json::from_str::<StationSettings>("{}").unwrap();
        serde_json::from_str::<Restrictions>("{}").unwrap();
        serde_json::from_str::<Sequence>("{}").unwrap();
        serde_json::from_str::<AdParams>("{}").unwrap();
        serde_json::from_str::<Id>("{}").unwrap();
        serde_json::from_str::<Enum>("{}").unwrap();
        serde_json::from_str::<DiscreteScale>("{}").unwrap();
        serde_json::from_str::<RestrictionValue>("{}").unwrap();
    }
}
