//! type-tolerant deserialization.
//!
//! the models already survive a field the api *stops* sending. this handles the
//! other half: a field whose *type* changes.
//!
//! that is not hypothetical. `Track.best` is a flag by every reading of its name
//! and its usual value, and the api sometimes sends an array there instead. a
//! parser that does not check would let the array land in the field unnoticed and
//! break something later, somewhere else; a parser that checks strictly — serde —
//! turns the same payload into a hard failure of the whole response.
//!
//! neither is right. [`deserialize`] takes the middle path: parse strictly, and
//! when one field's type does not fit, repair *that field only* and parse again.
//! a json float standing in for an integer is narrowed, losing nothing; anything
//! else is dropped. one changed field costs you one field, not the request — and
//! you are told which, so drift can be fixed rather than tolerated forever.
//!
//! wire [`ClientBuilder::on_field_repair`](crate::ClientBuilder::on_field_repair)
//! to hear about it, and filter on [`FieldRepair::is_lossy`] to separate real drift
//! from harmless number widening.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};

/// how many fields may be repaired before the response is treated as unusable.
///
/// a handful means upstream drift; dozens means the payload is not what this crate
/// thinks it is, and quietly returning a mostly-empty model would be worse than
/// failing.
pub const MAX_REPAIRS: usize = 16;

/// what had to be done to a field before the payload would parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repair {
    /// a json number that carried no fractional part was narrowed to an integer.
    ///
    /// this is the js-versus-rust impedance mismatch, not upstream drift: json has
    /// one number type, so an api written in javascript may serialize `0` as `0.0`
    /// for a field that is conceptually an integer. no information is lost.
    Narrowed,
    /// the value did not fit the field's type at all, so the field was dropped.
    ///
    /// this one is worth investigating: either the api changed or the model is wrong.
    Dropped,
}

/// a field that did not match its model, and what was done about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRepair {
    /// dotted path to the field, for example `tracks.results[0].best`.
    pub path: String,
    /// what serde objected to.
    pub reason: String,
    /// whether the value was salvaged or discarded.
    pub repair: Repair,
}

impl FieldRepair {
    /// whether the field was discarded rather than salvaged.
    ///
    /// filter on this to separate real drift from harmless number widening.
    #[must_use]
    pub fn is_lossy(&self) -> bool {
        self.repair == Repair::Dropped
    }
}

impl std::fmt::Display for FieldRepair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self.repair {
            Repair::Narrowed => "narrowed to an integer",
            Repair::Dropped => "dropped",
        };
        write!(f, "{} {what}: {}", self.path, self.reason)
    }
}

/// deserialize `value`, repairing individual fields whose type does not fit.
///
/// a json float with no fractional part is narrowed to the integer the model
/// expects; anything else that does not fit is dropped. returns the model and a
/// report of every field touched, in the order they were found — an empty report
/// means the payload matched exactly.
///
/// # errors
///
/// returns [`Error::InvalidResponse`] when the failure is not attributable to a
/// single field, or when more than [`MAX_REPAIRS`] fields would have to be touched.
pub fn deserialize<T: DeserializeOwned>(value: Value) -> Result<(T, Vec<FieldRepair>)> {
    let mut value = value;
    let mut repaired = Vec::new();

    loop {
        let mut track = serde_path_to_error::Track::new();
        let de = serde_path_to_error::Deserializer::new(&value, &mut track);

        match T::deserialize(de) {
            Ok(model) => return Ok((model, repaired)),
            Err(err) => {
                let path = track.path().to_string();
                let reason = err.to_string();

                // no path means the whole document is the wrong shape; there is no
                // single field to blame and nothing useful to repair.
                if path.is_empty() || path == "." {
                    return Err(Error::InvalidResponse(reason));
                }
                // the budget counts distinct *fields*, not repairs. one field
                // mistyped in the model is mistyped once per element on the
                // wire, so counting raw repairs made a single modelling error
                // in a list look like a wholly unrecognisable payload — and
                // failed the call the budget exists to keep alive.
                let distinct = repaired
                    .iter()
                    .map(|r: &FieldRepair| field_of(&r.path))
                    .chain(std::iter::once(field_of(&path)))
                    .collect::<std::collections::HashSet<_>>()
                    .len();

                if distinct > MAX_REPAIRS {
                    return Err(Error::InvalidResponse(format!(
                        "gave up after repairing {MAX_REPAIRS} distinct mistyped fields; last was {path}: {reason}"
                    )));
                }

                // repair the field across every element of the list at once.
                // serde reports one element at a time, so repairing only the
                // one it named would cost a full deserialize pass per element —
                // quadratic in the length of the list, and a liked-albums
                // response is thousands of them.
                let Some(repair) = repair_field(&mut value, &path) else {
                    // nothing changed, so retrying would spin on the same field
                    return Err(Error::InvalidResponse(format!("{path}: {reason}")));
                };
                repaired.push(FieldRepair {
                    path,
                    reason,
                    repair,
                });
            },
        }
    }
}

/// narrow a float to an integer, when it has no fractional part.
///
/// returns `false` when the value is not such a float, so the caller falls back
/// to dropping it.
fn narrow_value(cursor: &mut Value) -> bool {
    /// 2^63 — the exclusive bound of what an `i64` holds.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;

    let Some(float) = cursor.as_f64() else {
        return false;
    };
    if cursor.is_i64() || cursor.is_u64() {
        return false; // already an integer; the mismatch is something else
    }
    // only exact values: silently rounding a real fraction would be a lie
    if float.fract() != 0.0 || !float.is_finite() {
        return false;
    }
    // and only values an i64 can hold, so the cast below cannot saturate
    if !(-LIMIT..LIMIT).contains(&float) {
        return false;
    }

    // guarded above: integral, finite and in range, so this is exact
    #[allow(clippy::cast_possible_truncation)]
    let exact = float as i64;
    *cursor = Value::from(exact);
    true
}

/// the field a path names, with array positions collapsed.
///
/// `tracks[0].best` and `tracks[41].best` are the same modelling mistake seen
/// twice, and the repair budget has to treat them as one; otherwise a long
/// enough list turns any single wrong field into a failed request.
fn field_of(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_index = false;

    for c in path.chars() {
        match c {
            // `serde_path_to_error` writes sequence positions as `[7]`.
            '[' => in_index = true,
            ']' => in_index = false,
            _ if in_index => {},
            c => out.push(c),
        }
    }

    // a leading `.` is left behind when the path started with an index.
    out.trim_start_matches('.').to_owned()
}

/// repair the field at `path`, and every sibling of it that would fail the same
/// way.
///
/// "the same way" is judged by json type: the value serde rejected was, say, a
/// number where a string was wanted, so the other numbers under that field will
/// be rejected identically while the strings are already fine and are left
/// alone. that keeps the fan-out precise — one wrong field is repaired
/// everywhere at once, and a genuinely mixed list only loses the entries that
/// really do not fit.
///
/// returns `None` when nothing changed, which is the caller's signal to stop.
fn repair_field(root: &mut Value, path: &str) -> Option<Repair> {
    let offender = kind_of(resolve(root, path)?);

    let mut narrowed = false;
    let mut dropped = false;

    for cursor in resolve_siblings(root, path) {
        if kind_of(cursor) != offender {
            continue;
        }
        // prefer salvaging the value over discarding it
        if narrow_value(cursor) {
            narrowed = true;
        } else if !cursor.is_null() {
            *cursor = Value::Null;
            dropped = true;
        }
    }

    match (dropped, narrowed) {
        (true, _) => Some(Repair::Dropped),
        (false, true) => Some(Repair::Narrowed),
        (false, false) => None,
    }
}

/// the json type of a value, for deciding which siblings share its problem.
fn kind_of(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}

/// every value `path` names when each array index is read as "all elements".
fn resolve_siblings<'v>(root: &'v mut Value, path: &str) -> Vec<&'v mut Value> {
    fn walk<'v>(cursor: &'v mut Value, segments: &[Segment], out: &mut Vec<&'v mut Value>) {
        let Some((head, tail)) = segments.split_first() else {
            out.push(cursor);
            return;
        };

        match head {
            Segment::Key(key) => {
                if let Some(next) = cursor.as_object_mut().and_then(|o| o.get_mut(key.as_str())) {
                    walk(next, tail, out);
                }
            },
            // the fan-out: the index serde happened to report is ignored in
            // favour of every position in the same array.
            Segment::Index(_) => {
                if let Some(array) = cursor.as_array_mut() {
                    for item in array {
                        walk(item, tail, out);
                    }
                }
            },
        }
    }

    let mut out = Vec::new();
    walk(root, &segments(path), &mut out);
    out
}

/// deserialize, discarding the report of repaired fields.
///
/// # errors
///
/// see [`deserialize`].
pub fn deserialize_lossy<T: DeserializeOwned>(value: Value) -> Result<T> {
    deserialize(value).map(|(model, _)| model)
}

/// walk to the value at `path`, or `None` when it does not resolve.
fn resolve<'v>(root: &'v mut Value, path: &str) -> Option<&'v mut Value> {
    let mut cursor = root;
    for segment in segments(path) {
        cursor = match segment {
            Segment::Key(key) => cursor.as_object_mut()?.get_mut(&key)?,
            Segment::Index(index) => cursor.as_array_mut()?.get_mut(index)?,
        };
    }
    Some(cursor)
}

enum Segment {
    Key(String),
    Index(usize),
}

/// split a `serde_path_to_error` path into segments.
///
/// the format is dotted keys with bracketed indices: `tracks.results[0].best`.
/// keys containing a `.` or a `[` cannot be represented and are not produced by
/// this api.
fn segments(path: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    for part in path.split('.') {
        let (name, rest) = match part.find('[') {
            Some(at) => (&part[..at], &part[at..]),
            None => (part, ""),
        };
        if !name.is_empty() {
            out.push(Segment::Key(name.to_owned()));
        }
        for index in rest.split(']').filter(|s| !s.is_empty()) {
            if let Ok(index) = index.trim_start_matches('[').parse() {
                out.push(Segment::Index(index));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Search, Track};

    #[test]
    fn a_matching_payload_drops_nothing() {
        let raw = serde_json::json!({ "id": 42, "title": "t", "available": true });
        let (track, dropped): (Track, _) = deserialize(raw).unwrap();
        assert_eq!(track.title.as_deref(), Some("t"));
        assert_eq!(track.available, Some(true));
        assert!(dropped.is_empty());
    }

    #[test]
    fn a_boolean_that_became_an_array_costs_only_that_field() {
        // the exact upstream shape that made a whole search fail
        let raw = serde_json::json!({
            "id": 42,
            "title": "kept",
            "available": true,
            "best": ["not", "a", "boolean"],
        });

        let (track, dropped): (Track, _) = deserialize(raw).unwrap();

        assert_eq!(
            track.title.as_deref(),
            Some("kept"),
            "the rest of the track survives"
        );
        assert_eq!(track.available, Some(true));
        assert_eq!(
            track.best, None,
            "the mistyped field is dropped, not guessed at"
        );

        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].path, "best");
        assert_eq!(dropped[0].repair, Repair::Dropped);
        assert!(dropped[0].is_lossy());
        assert!(
            dropped[0].reason.contains("boolean"),
            "{}",
            dropped[0].reason
        );
    }

    #[test]
    fn a_json_float_standing_in_for_an_integer_is_narrowed_not_dropped() {
        // json has one number type, so a js-authored api serializes 299 as 299.0
        let raw = serde_json::json!({
            "autoRenewable": [{ "product": { "price": { "amount": 0.0, "currency": "RUB" } } }],
        });
        let (subscription, repaired): (crate::models::Subscription, _) = deserialize(raw).unwrap();

        // for a field the model still wants as an integer, the value survives
        let raw_int = serde_json::json!({ "id": 1, "durationMs": 210_000.0 });
        let (track, track_repairs): (Track, _) = deserialize(raw_int).unwrap();
        assert_eq!(
            track.duration_ms,
            Some(210_000),
            "the value is kept, not dropped"
        );
        assert_eq!(track_repairs[0].repair, Repair::Narrowed);
        assert!(!track_repairs[0].is_lossy(), "narrowing loses nothing");

        // price.amount is a float in the model, so it needs no repair at all
        assert!(repaired.is_empty());
        let amount = subscription.auto_renewable[0]
            .product
            .as_ref()
            .unwrap()
            .price
            .as_ref()
            .unwrap()
            .amount;
        assert_eq!(amount, Some(0.0));
    }

    #[test]
    fn a_float_with_a_real_fraction_is_dropped_rather_than_rounded() {
        // silently rounding 1.5 to 1 or 2 would be a lie about what the api said
        let raw = serde_json::json!({ "id": 1, "durationMs": 210_000.5 });
        let (track, repaired): (Track, _) = deserialize(raw).unwrap();
        assert_eq!(track.duration_ms, None);
        assert_eq!(repaired[0].repair, Repair::Dropped);
    }

    #[test]
    fn a_mistyped_field_deep_inside_a_search_response_is_reported_with_its_path() {
        let raw = serde_json::json!({
            "text": "query",
            "tracks": {
                "total": 2,
                "results": [
                    { "id": 1, "title": "first" },
                    { "id": 2, "title": "second", "explicit": [] },
                ],
            },
        });

        let (search, dropped): (Search, _) = deserialize(raw).unwrap();

        assert_eq!(search.track_results().len(), 2, "both tracks still arrive");
        assert_eq!(search.track_results()[1].title.as_deref(), Some("second"));
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].path, "tracks.results[1].explicit");
        assert_eq!(dropped[0].repair, Repair::Dropped);
    }

    #[test]
    fn several_mistyped_fields_are_all_dropped_and_all_reported() {
        let raw = serde_json::json!({
            "id": 1,
            "available": ["x"],
            "explicit": 7,
            "title": "kept",
        });
        let (track, dropped): (Track, _) = deserialize(raw).unwrap();
        assert_eq!(track.title.as_deref(), Some("kept"));
        assert_eq!(dropped.len(), 2);
        let paths: Vec<&str> = dropped.iter().map(|d| d.path.as_str()).collect();
        assert!(paths.contains(&"available"));
        assert!(paths.contains(&"explicit"));
    }

    #[test]
    fn a_mistyped_field_inside_a_nested_object_is_handled() {
        let raw = serde_json::json!({ "id": 1, "major": { "id": 2, "name": ["a"] } });
        let (track, dropped): (Track, _) = deserialize(raw).unwrap();
        assert_eq!(track.major.as_ref().unwrap().id, Some(2));
        assert_eq!(track.major.as_ref().unwrap().name, None);
        assert_eq!(dropped[0].path, "major.name");
    }

    #[test]
    fn a_wholly_wrong_document_still_fails() {
        // a bare string where a track was expected is not one bad field
        let err = deserialize::<Track>(serde_json::json!("not a track")).unwrap_err();
        assert!(matches!(err, Error::InvalidResponse(_)));
    }

    #[test]
    fn a_payload_needing_too_many_drops_fails_rather_than_arriving_empty() {
        // 17 mistyped fields: past the point where the payload is plausibly ours
        let mut raw = serde_json::Map::new();
        for field in [
            "available",
            "explicit",
            "best",
            "canPublish",
            "availableAsRbt",
            "lyricsAvailable",
            "rememberPosition",
            "isSuitableForChildren",
            "availableForPremiumUsers",
            "availableFullWithoutPermission",
            "durationMs",
            "fileSize",
            "previewDurationMs",
            "title",
            "type",
            "coverUri",
            "storageDir",
        ] {
            raw.insert(field.into(), serde_json::json!(["wrong"]));
        }
        let err = deserialize::<Track>(Value::Object(raw)).unwrap_err();
        assert!(matches!(err, Error::InvalidResponse(_)));
        assert!(err.to_string().contains("gave up"), "{err}");
    }

    #[test]
    fn the_lossy_helper_hides_the_report() {
        let raw = serde_json::json!({ "id": 1, "best": [] });
        let track: Track = deserialize_lossy(raw).unwrap();
        assert_eq!(track.best, None);
    }

    #[test]
    fn paths_split_into_keys_and_indices() {
        let parsed: Vec<String> = segments("tracks.results[1].albums[0].title")
            .iter()
            .map(|s| match s {
                Segment::Key(k) => k.clone(),
                Segment::Index(i) => format!("#{i}"),
            })
            .collect();
        assert_eq!(
            parsed,
            vec!["tracks", "results", "#1", "albums", "#0", "title"]
        );
    }

    #[test]
    fn a_flag_field_accepts_the_list_the_api_actually_sends() {
        // hasOptions: the name and the upstream types say boolean, the api disagrees
        use crate::models::{FlagOrList, Status};

        let as_list: Status =
            deserialize_lossy(serde_json::json!({ "hasOptions": ["option-a", "option-b"] }))
                .unwrap();
        let options = as_list.has_options.unwrap();
        assert!(options.is_set());
        assert_eq!(options.list().len(), 2);

        let as_flag: Status = deserialize_lossy(serde_json::json!({ "hasOptions": true })).unwrap();
        assert!(as_flag.has_options.unwrap().is_set());

        let empty_list: Status =
            deserialize_lossy(serde_json::json!({ "hasOptions": [] })).unwrap();
        assert!(
            !empty_list.has_options.unwrap().is_set(),
            "an empty list is not set"
        );

        assert!(!FlagOrList::Flag(false).is_set());
        assert!(FlagOrList::Flag(true).list().is_empty());
    }

    #[test]
    fn repairing_reports_whether_it_changed_anything() {
        let mut value = serde_json::json!({ "a": { "b": [1, 2] } });
        assert_eq!(repair_field(&mut value, "a.b[0]"), Some(Repair::Dropped));

        // repeating it changes nothing, which is the caller's signal to stop
        assert_eq!(repair_field(&mut value, "a.b[0]"), None);
        assert_eq!(repair_field(&mut value, "a.missing"), None);
        assert_eq!(repair_field(&mut value, "a.b[9]"), None);
    }

    #[test]
    fn one_bad_element_repairs_the_whole_list_in_a_single_pass() {
        // the point of the fan-out: serde reports element 0, but the field is
        // wrong in every element, and repairing them one deserialize at a time
        // is quadratic in the length of the list.
        let mut value = serde_json::json!({ "items": [{ "id": 1 }, { "id": 2 }, { "id": 3 }] });

        assert_eq!(
            repair_field(&mut value, "items[0].id"),
            Some(Repair::Dropped)
        );
        assert_eq!(
            value,
            serde_json::json!({ "items": [{ "id": null }, { "id": null }, { "id": null }] })
        );
    }

    #[test]
    fn a_sibling_of_a_different_json_type_is_left_alone() {
        // a genuinely mixed list only loses the entries that really do not fit:
        // the strings here are what the model wanted, so they survive.
        let mut value = serde_json::json!({ "items": [{ "id": 1 }, { "id": "ok" }, { "id": 3 }] });

        assert_eq!(
            repair_field(&mut value, "items[0].id"),
            Some(Repair::Dropped)
        );
        assert_eq!(
            value,
            serde_json::json!({ "items": [{ "id": null }, { "id": "ok" }, { "id": null }] })
        );
    }

    #[test]
    fn a_field_path_ignores_the_array_positions_it_passes_through() {
        assert_eq!(field_of("[16].id"), "id");
        assert_eq!(field_of("tracks.results[0].best"), "tracks.results.best");
        assert_eq!(field_of("plain.path"), "plain.path");
    }

    #[test]
    fn the_budget_counts_distinct_fields_not_repeated_ones() {
        // one field mistyped in the model is mistyped once per element on the
        // wire. counting raw repairs made that look like an unrecognisable
        // payload and failed the very call the budget exists to keep alive.
        #[derive(serde::Deserialize)]
        struct Item {
            #[serde(default)]
            #[allow(dead_code)]
            id: Option<String>,
        }

        let raw: Value = serde_json::from_str(&format!(
            "[{}]",
            (0..MAX_REPAIRS * 8)
                .map(|i| format!(r#"{{"id":{i}}}"#))
                .collect::<Vec<_>>()
                .join(",")
        ))
        .unwrap();

        let (items, repaired): (Vec<Item>, _) = deserialize(raw).unwrap();

        assert_eq!(items.len(), MAX_REPAIRS * 8);
        // and the whole list was fixed in one pass, so one repair is reported
        assert_eq!(repaired.len(), 1);
    }
}
