//! playlist edit diffs.
//!
//! the playlist-change endpoint takes a json array of operations rather than a
//! rest-style body. [`Difference`] accumulates those operations and serializes
//! them into the `diff` parameter.

use serde::Serialize;

/// a track reference used when inserting into a playlist.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffTrack {
    /// track id.
    pub id: i64,
    /// album id the track belongs to.
    pub album_id: i64,
}

impl DiffTrack {
    /// reference a track by its own id and its album's.
    #[must_use]
    pub fn new(id: i64, album_id: i64) -> Self {
        Self { id, album_id }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum Operation {
    Insert { at: usize, tracks: Vec<DiffTrack> },
    Delete { from: usize, to: usize },
}

/// accumulates playlist edit operations and serializes them to the `diff` string.
///
/// # examples
///
/// ```
/// use yamuse::{Difference, DiffTrack};
///
/// let diff = Difference::new()
///     .insert(0, [DiffTrack::new(42, 1001)])
///     .delete(3, 5)
///     .to_json();
///
/// assert!(diff.starts_with(r#"[{"op":"insert","at":0"#));
/// ```
#[derive(Debug, Default, Clone)]
pub struct Difference {
    operations: Vec<Operation>,
}

impl Difference {
    /// start an empty diff.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// remove the tracks in the half-open range `[from, to)`.
    #[must_use]
    pub fn delete(mut self, from: usize, to: usize) -> Self {
        self.operations.push(Operation::Delete { from, to });
        self
    }

    /// insert tracks at the given index.
    #[must_use]
    pub fn insert(mut self, at: usize, tracks: impl IntoIterator<Item = DiffTrack>) -> Self {
        self.operations.push(Operation::Insert {
            at,
            tracks: tracks.into_iter().collect(),
        });
        self
    }

    /// whether no operations were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// serialize the accumulated operations to the `diff` string.
    #[must_use]
    pub fn to_json(&self) -> String {
        // every variant is a plain struct of numbers, so this cannot fail.
        serde_json::to_string(&self.operations).unwrap_or_else(|_| "[]".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_insert_serializes_with_its_track_references() {
        let json = Difference::new()
            .insert(0, [DiffTrack::new(42, 1001)])
            .to_json();
        assert_eq!(
            json,
            r#"[{"op":"insert","at":0,"tracks":[{"id":42,"albumId":1001}]}]"#
        );
    }

    #[test]
    fn a_delete_serializes_as_a_half_open_range() {
        assert_eq!(
            Difference::new().delete(3, 5).to_json(),
            r#"[{"op":"delete","from":3,"to":5}]"#
        );
    }

    #[test]
    fn operations_keep_the_order_they_were_added_in() {
        let json = Difference::new()
            .delete(0, 1)
            .insert(0, [DiffTrack::new(1, 2), DiffTrack::new(3, 4)])
            .to_json();
        let ops: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0]["op"], "delete");
        assert_eq!(ops[1]["op"], "insert");
        assert_eq!(ops[1]["tracks"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn an_empty_diff_serializes_to_an_empty_array() {
        let diff = Difference::new();
        assert!(diff.is_empty());
        assert_eq!(diff.to_json(), "[]");
    }

    #[test]
    fn an_insert_of_no_tracks_is_still_a_recorded_operation() {
        let diff = Difference::new().insert(0, []);
        assert!(!diff.is_empty());
        assert_eq!(diff.to_json(), r#"[{"op":"insert","at":0,"tracks":[]}]"#);
    }
}
