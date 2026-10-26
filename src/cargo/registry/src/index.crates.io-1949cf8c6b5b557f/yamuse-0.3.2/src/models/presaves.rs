//! album pre-saves.

use serde::{Deserialize, Serialize};

use super::album::Album;

/// a user's pre-saved albums, split into upcoming and already-released.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Presaves {
    /// albums not yet released.
    pub upcoming_albums: Vec<Album>,
    /// albums that have since been released.
    pub released_albums: Vec<Album>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presaves_split_upcoming_from_released() {
        let presaves: Presaves = serde_json::from_str(
            r#"{"upcomingAlbums":[{"id":1,"millisecondsUntilRelease":86400000}],"releasedAlbums":[{"id":2}]}"#,
        )
        .unwrap();
        assert_eq!(
            presaves.upcoming_albums[0].milliseconds_until_release,
            Some(86_400_000)
        );
        assert_eq!(presaves.released_albums.len(), 1);
    }

    #[test]
    fn presaves_survive_an_empty_object() {
        let empty: Presaves = serde_json::from_str("{}").unwrap();
        assert!(empty.upcoming_albums.is_empty());
        assert!(empty.released_albums.is_empty());
    }
}
