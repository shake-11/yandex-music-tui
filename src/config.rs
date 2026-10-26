//! Application configuration and XDG paths.
//!
//! The config intentionally lives at `$XDG_CONFIG_HOME/yandex-tui/config.toml`
//! and the audio cache at `$XDG_CACHE_HOME/yandex-tui/audio`. Tokens are optional
//! here: `YANDEX_MUSIC_TOKEN` / `YM_TOKEN` always win, which keeps secrets out of
//! dotfiles for users who prefer environment managers or password-store wrappers.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const APP_DIR: &str = "yandex-tui";

/// Errors while resolving, reading, or writing the configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not locate your home directory; set HOME or XDG_CONFIG_HOME")]
    NoHome,
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("could not serialise configuration: {0}")]
    Serialise(#[from] toml::ser::Error),
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Lossy playback quality requested from the private API.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioQuality {
    /// Smallest files, useful on metered links.
    Low,
    /// The everyday sweet spot.
    Normal,
    /// The default: high-quality MP3 without needless bandwidth.
    #[default]
    High,
    /// The highest MP3 variant Yandex serves.
    Best,
}

impl AudioQuality {
    /// Target bitrate in kbit/s. The closest available MP3 variant is chosen.
    #[must_use]
    pub const fn target_kbps(self) -> i64 {
        match self {
            Self::Low => 64,
            Self::Normal => 128,
            Self::High => 192,
            Self::Best => 320,
        }
    }
}

/// Stored OAuth credentials. The access token may also come from the environment.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// OAuth access token (a Yandex Music token also works).
    pub access_token: String,
    /// Refresh token returned by the device flow.
    pub refresh_token: String,
    /// Unix timestamp at which the access token expires, when known.
    pub expires_at: i64,
}

impl AuthConfig {
    /// Remove all credentials.
    pub fn clear(&mut self) {
        self.access_token.clear();
        self.refresh_token.clear();
        self.expires_at = 0;
    }

    /// Store a fresh token response.
    pub fn store(&mut self, access_token: String, refresh_token: Option<String>, expires_at: i64) {
        self.access_token = access_token;
        if let Some(refresh_token) = refresh_token {
            self.refresh_token = refresh_token;
        }
        self.expires_at = expires_at;
    }
}

/// Audio and transport preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackConfig {
    /// Linear volume in the inclusive range 0.0..=1.0.
    pub volume: f32,
    /// Requested streaming quality.
    pub quality: AudioQuality,
    /// How far Ctrl+Left / Ctrl+Right jump.
    pub seek_step_seconds: u64,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            volume: 0.72,
            quality: AudioQuality::High,
            seek_step_seconds: 10,
        }
    }
}

/// Interface preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Enable the short track-change pulse and animated indicators.
    pub animations: bool,
    /// Fetch and draw album art as portable half-block art.
    pub cover_art: bool,
    /// Width below which the labelled sidebar folds into an icon rail.
    pub compact_sidebar_below: u16,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            animations: true,
            cover_art: true,
            compact_sidebar_below: 116,
        }
    }
}

/// Local cache preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Keep the offline audio cache between runs. When false, playback still
    /// uses a temporary session cache, but `d` cannot pin new tracks.
    pub enabled: bool,
    /// Optional cache directory override. Relative paths resolve from the current directory.
    pub directory: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: String::new(),
        }
    }
}

/// The complete TOML configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auth: AuthConfig,
    pub playback: PlaybackConfig,
    pub ui: UiConfig,
    pub cache: CacheConfig,
}

/// Resolved XDG locations used by the application.
#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
    pub audio_dir: PathBuf,
}

impl Paths {
    /// Resolve XDG paths, honouring `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `HOME`.
    pub fn discover() -> Result<Self, ConfigError> {
        let home = env::var_os("HOME").map(PathBuf::from);
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| home.as_ref().map(|home| home.join(".config")))
            .ok_or(ConfigError::NoHome)?;
        let cache_home = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| home.as_ref().map(|home| home.join(".cache")))
            .ok_or(ConfigError::NoHome)?;

        Ok(Self::from_roots(&config_home, &cache_home))
    }

    /// Build paths from explicit roots. Kept pure so it is testable without mutating env.
    #[must_use]
    pub fn from_roots(config_home: &Path, cache_home: &Path) -> Self {
        let config_dir = config_home.join(APP_DIR);
        let cache_dir = cache_home.join(APP_DIR);
        Self {
            config_file: config_dir.join("config.toml"),
            audio_dir: cache_dir.join("audio"),
            config_dir,
            cache_dir,
        }
    }

    /// Resolve the audio cache directory, applying an optional config override.
    #[must_use]
    pub fn audio_dir_for(&self, config: &Config) -> PathBuf {
        let override_dir = config.cache.directory.trim();
        if override_dir.is_empty() {
            self.audio_dir.clone()
        } else {
            let path = PathBuf::from(override_dir);
            if path.is_absolute() {
                path
            } else {
                self.config_dir.join(path)
            }
        }
    }
}

impl Config {
    /// Load the config from a path, falling back to defaults when it does not exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                path: path.to_owned(),
                source,
            }),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            }),
        }
    }

    /// Persist the config atomically and restrict file permissions to the owner.
    ///
    /// The file may contain a refresh token, so mode 0600 matters even though it
    /// is not a substitute for a keyring.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let Some(parent) = path.parent() else {
            return Err(ConfigError::Write {
                path: path.to_owned(),
                source: io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"),
            });
        };
        fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: path.to_owned(),
            source,
        })?;

        let serialised = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, serialised).map_err(|source| ConfigError::Write {
            path: tmp.clone(),
            source,
        })?;
        set_private_permissions(&tmp)?;
        fs::rename(&tmp, path).map_err(|source| ConfigError::Write {
            path: path.to_owned(),
            source,
        })?;
        set_private_permissions(path)?;
        Ok(())
    }

    /// The access token from the environment, if present and non-empty.
    #[must_use]
    pub fn token_from_env() -> Option<String> {
        ["YANDEX_MUSIC_TOKEN", "YM_TOKEN"]
            .iter()
            .find_map(|key| env::var(key).ok())
            .map(|token| token.trim().to_owned())
            .filter(|token| !token.is_empty())
    }

    /// Normalise values loaded from disk.
    pub fn sanitise(&mut self) {
        self.playback.volume = self.playback.volume.clamp(0.0, 1.0);
        self.playback.seek_step_seconds = self.playback.seek_step_seconds.clamp(1, 120);
        self.ui.compact_sidebar_below = self.ui.compact_sidebar_below.clamp(80, 240);
        self.auth.access_token = self.auth.access_token.trim().to_owned();
        self.auth.refresh_token = self.auth.refresh_token.trim().to_owned();
        self.cache.directory = self.cache.directory.trim().to_owned();
    }
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), ConfigError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        ConfigError::Write {
            path: path.to_owned(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_roots_are_folded_into_application_paths() {
        let paths = Paths::from_roots(Path::new("/tmp/conf"), Path::new("/tmp/cache"));
        assert_eq!(
            paths.config_file,
            PathBuf::from("/tmp/conf/yandex-tui/config.toml")
        );
        assert_eq!(
            paths.audio_dir,
            PathBuf::from("/tmp/cache/yandex-tui/audio")
        );
    }

    #[test]
    fn cache_override_can_be_absolute_or_config_relative() {
        let paths = Paths::from_roots(Path::new("/tmp/conf"), Path::new("/tmp/cache"));
        let mut config = Config::default();
        assert_eq!(paths.audio_dir_for(&config), paths.audio_dir);

        config.cache.directory = "tracks".into();
        assert_eq!(
            paths.audio_dir_for(&config),
            PathBuf::from("/tmp/conf/yandex-tui/tracks")
        );

        config.cache.directory = "/var/tmp/ytui".into();
        assert_eq!(paths.audio_dir_for(&config), PathBuf::from("/var/tmp/ytui"));
    }

    #[test]
    fn defaults_are_safe_and_parse_from_an_empty_file() {
        let config: Config = toml::from_str("").expect("empty config is valid");
        assert!((config.playback.volume - 0.72).abs() < f32::EPSILON);
        assert_eq!(config.playback.quality, AudioQuality::High);
        assert!(config.ui.animations);
        assert!(config.cache.enabled);
    }

    #[test]
    fn sanitise_clamps_untrusted_values() {
        let mut config: Config = toml::from_str(
            r#"
            [playback]
            volume = 8.0
            seek_step_seconds = 0

            [auth]
            access_token = "  secret  "
            "#,
        )
        .unwrap();
        config.sanitise();
        assert_eq!(config.playback.volume, 1.0);
        assert_eq!(config.playback.seek_step_seconds, 1);
        assert_eq!(config.auth.access_token, "secret");
    }

    #[test]
    fn example_config_is_valid_and_matches_defaults() {
        let config: Config =
            toml::from_str(include_str!("../config.example.toml")).expect("example config parses");
        assert!((config.playback.volume - 0.72).abs() < f32::EPSILON);
        assert_eq!(config.playback.quality, AudioQuality::High);
        assert_eq!(config.playback.seek_step_seconds, 10);
        assert!(config.ui.animations);
        assert!(config.ui.cover_art);
        assert_eq!(config.ui.compact_sidebar_below, 116);
        assert!(config.cache.enabled);
        assert!(config.cache.directory.is_empty());
        assert!(config.auth.access_token.is_empty());
    }

    #[test]
    fn quality_maps_to_a_sensible_target() {
        assert_eq!(AudioQuality::Low.target_kbps(), 64);
        assert_eq!(AudioQuality::Best.target_kbps(), 320);
    }
}
