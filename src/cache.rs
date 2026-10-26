//! Local audio cache.
//!
//! A track only appears in the index after a complete file has been renamed into
//! place. Partial transfers use a sibling `.part` file, so a crash can never make
//! a truncated MP3 look playable.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::model::Track;

/// Thread-safe index of fully downloaded tracks.
#[derive(Debug, Clone)]
pub struct AudioCache {
    directory: PathBuf,
    keys: Arc<Mutex<HashSet<String>>>,
}

impl AudioCache {
    /// Open (and create) the cache directory, indexing all valid MP3 files.
    pub fn open(directory: impl Into<PathBuf>) -> io::Result<Self> {
        let directory = directory.into();
        fs::create_dir_all(&directory)?;
        let mut keys = HashSet::new();
        for entry in fs::read_dir(&directory)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("mp3") {
                continue;
            }
            let valid = entry
                .metadata()
                .map(|metadata| metadata.len() > 16 * 1024)
                .unwrap_or(false);
            if !valid {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                keys.insert(stem.to_owned());
            }
        }
        Ok(Self {
            directory,
            keys: Arc::new(Mutex::new(keys)),
        })
    }

    /// Directory containing audio files.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Destination for a cached track.
    #[must_use]
    pub fn path_for(&self, track: &Track) -> PathBuf {
        self.path_for_id(&track.cache_key())
    }

    /// Destination for a raw cache key.
    #[must_use]
    pub fn path_for_id(&self, key: &str) -> PathBuf {
        self.directory.join(format!("{key}.mp3"))
    }

    /// Whether the track is fully cached.
    #[must_use]
    pub fn contains(&self, track: &Track) -> bool {
        self.keys
            .lock()
            .map(|keys| keys.contains(&track.cache_key()))
            .unwrap_or(false)
    }

    /// Record a track after it was renamed into place.
    pub fn mark_complete(&self, track: &Track) {
        if let Ok(mut keys) = self.keys.lock() {
            keys.insert(track.cache_key());
        }
    }

    /// Current number of ready files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.lock().map(|keys| keys.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: "T".into(),
            artist: "A".into(),
            album: None,
            year: None,
            duration_ms: 1000,
            cover_url: None,
            explicit: false,
            available: true,
        }
    }

    #[test]
    fn only_complete_files_are_indexed() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("ytui-cache-test-{}-{unique}", std::process::id()));

        let cache = AudioCache::open(&dir).unwrap();
        fs::write(cache.path_for(&track("42")), vec![0u8; 20 * 1024]).unwrap();
        fs::write(dir.join("tiny.mp3"), b"no").unwrap();
        fs::write(dir.join("partial.part"), vec![0u8; 20 * 1024]).unwrap();

        let reopened = AudioCache::open(&dir).unwrap();
        assert!(reopened.contains(&track("42")));
        assert!(!reopened.contains(&track("tiny")));
        assert_eq!(reopened.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unsafe_ids_become_safe_keys() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("ytui-cache-key-{}-{unique}", std::process::id()));
        let cache = AudioCache::open(&dir).unwrap();
        let path = cache.path_for(&track("../escape"));
        assert_eq!(path.parent(), Some(dir.as_path()));
        let _ = fs::remove_dir_all(dir);
    }
}
