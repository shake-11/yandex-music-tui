//! api drift detection.
//!
//! this wraps a private api that gains and loses fields without notice. the
//! models tolerate that by design — an unknown key is ignored rather than fatal —
//! but "tolerated" means "invisible", and a field this crate silently drops is a
//! field you cannot use.
//!
//! [`unmapped_fields`] makes it visible: give it the raw response and the model
//! parsed from it, and it reports every key the model did not consume. run it in
//! a staging job or a test against recorded payloads to notice a new field the
//! week it appears instead of the year you need it.
//!
//! ```
//! use yamuse::drift::unmapped_fields;
//! use yamuse::models::Track;
//!
//! let raw: serde_json::Value = serde_json::from_str(
//!     r#"{"id":42,"title":"t","brandNewUpstreamField":"surprise"}"#,
//! )?;
//! let track: Track = serde_json::from_value(raw.clone())?;
//!
//! assert_eq!(unmapped_fields(&raw, &track)?, vec!["brandNewUpstreamField"]);
//! # Ok::<(), yamuse::Error>(())
//! ```

use serde::Serialize;
use serde_json::Value;

use crate::error::Result;

/// report every key present in `raw` that `model` did not map, as dotted paths.
///
/// paths use `.` for object keys and `[i]` for array indices, so a field buried
/// in a list of playlists comes back as `blocks[0].entities[2].newField`.
///
/// # errors
///
/// returns [`Error::InvalidResponse`](crate::Error::InvalidResponse) when the
/// model cannot be serialized back to json.
pub fn unmapped_fields<T: Serialize>(raw: &Value, model: &T) -> Result<Vec<String>> {
    let mapped = serde_json::to_value(model)?;
    let mut found = Vec::new();
    diff(raw, &mapped, "", &mut found);
    found.sort();
    Ok(found)
}

/// walk `raw` against `mapped`, collecting keys the latter lacks.
fn diff(raw: &Value, mapped: &Value, path: &str, found: &mut Vec<String>) {
    match (raw, mapped) {
        (Value::Object(raw_map), Value::Object(mapped_map)) => {
            for (key, raw_child) in raw_map {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match mapped_map.get(key) {
                    Some(mapped_child) => diff(raw_child, mapped_child, &child_path, found),
                    None => found.push(child_path),
                }
            }
        },
        (Value::Array(raw_items), Value::Array(mapped_items)) => {
            // compare element-wise; a model that kept fewer elements than the api
            // sent is a filtering decision, not drift, so extra elements are skipped
            for (index, (raw_item, mapped_item)) in
                raw_items.iter().zip(mapped_items.iter()).enumerate()
            {
                diff(raw_item, mapped_item, &format!("{path}[{index}]"), found);
            }
        },
        // a scalar, or a shape mismatch: nothing nested left to compare. a mismatch
        // is not reported — it means the field *is* mapped, just differently typed,
        // which serde would already have rejected if it mattered.
        _ => {},
    }
}

/// whether `model` mapped every key in `raw`.
///
/// # errors
///
/// see [`unmapped_fields`].
pub fn is_fully_mapped<T: Serialize>(raw: &Value, model: &T) -> Result<bool> {
    Ok(unmapped_fields(raw, model)?.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Album, Landing, Track};

    #[test]
    fn a_fully_mapped_payload_reports_nothing() {
        let raw: Value = serde_json::from_str(r#"{"id":42,"title":"t","available":true}"#).unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert!(unmapped_fields(&raw, &track).unwrap().is_empty());
        assert!(is_fully_mapped(&raw, &track).unwrap());
    }

    #[test]
    fn a_new_top_level_field_is_reported() {
        let raw: Value =
            serde_json::from_str(r#"{"id":42,"somethingNew":1,"alsoNew":"x"}"#).unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            unmapped_fields(&raw, &track).unwrap(),
            vec!["alsoNew", "somethingNew"]
        );
        assert!(!is_fully_mapped(&raw, &track).unwrap());
    }

    #[test]
    fn a_new_field_inside_a_nested_object_is_reported_with_its_path() {
        let raw: Value =
            serde_json::from_str(r#"{"id":1,"major":{"id":2,"name":"m","newInner":true}}"#)
                .unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            unmapped_fields(&raw, &track).unwrap(),
            vec!["major.newInner"]
        );
    }

    #[test]
    fn a_new_field_inside_a_list_element_carries_its_index() {
        let raw: Value =
            serde_json::from_str(r#"{"id":1,"artists":[{"id":10},{"id":11,"unexpected":"here"}]}"#)
                .unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            unmapped_fields(&raw, &track).unwrap(),
            vec!["artists[1].unexpected"]
        );
    }

    #[test]
    fn drift_is_found_several_levels_deep() {
        let raw: Value = serde_json::from_str(
            r#"{"blocks":[{"id":"b","entities":[{"type":"album","futureKey":42}]}]}"#,
        )
        .unwrap();
        let landing: Landing = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            unmapped_fields(&raw, &landing).unwrap(),
            vec!["blocks[0].entities[0].futureKey"]
        );
    }

    #[test]
    fn an_absent_optional_field_is_not_mistaken_for_drift() {
        // a model field the api simply did not send must not be reported: drift is
        // about keys the api has that the model lacks, not the reverse
        let raw: Value = serde_json::from_str(r#"{"id":42}"#).unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert!(unmapped_fields(&raw, &track).unwrap().is_empty());
    }

    #[test]
    fn free_form_values_swallow_their_contents_rather_than_reporting_them() {
        // `trailer` is kept as a raw Value, so everything inside it is "mapped"
        let raw: Value =
            serde_json::from_str(r#"{"id":1,"trailer":{"anything":{"nested":true}}}"#).unwrap();
        let album: Album = serde_json::from_value(raw.clone()).unwrap();
        assert!(unmapped_fields(&raw, &album).unwrap().is_empty());
    }

    #[test]
    fn an_empty_payload_reports_nothing() {
        let raw: Value = serde_json::json!({});
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert!(unmapped_fields(&raw, &track).unwrap().is_empty());
    }

    #[test]
    fn the_same_new_key_in_several_list_elements_is_reported_once_per_element() {
        let raw: Value =
            serde_json::from_str(r#"{"id":1,"artists":[{"id":1,"nw":1},{"id":2,"nw":2}]}"#)
                .unwrap();
        let track: Track = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            unmapped_fields(&raw, &track).unwrap(),
            vec!["artists[0].nw", "artists[1].nw"]
        );
    }
}
