//! Runtime orchestration and the terminal event loop.
//!
//! Everything asynchronous lives here: authentication, catalogue and audio
//! downloads, cover art, lyrics, and the dedicated audio thread. The UI thread
//! receives outcomes and redraws; neither network failures nor audio-device
//! failures are allowed to unwind the terminal session.

use std::io::{self, IsTerminal, Stdout};
use std::panic;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::event::{Event as TerminalEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::{interval, MissedTickBehavior};

use crate::api::auth::{self};
use crate::api::demo;
use crate::api::service::ApiService;
use crate::app::{Action, App, CollectionRequest, Mode};
use crate::cache::AudioCache;
use crate::config::{AudioQuality, Config, ConfigError, Paths};
use crate::model::{LoadedCollection, Profile, WaveBatch, WaveFeedbackKind};
use crate::player::{AudioPlayer, PlayerCommand, PlayerEvent};

/// Command-line options that affect runtime startup.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub demo: bool,
    pub login: bool,
    pub token: Option<String>,
    pub config_path: Option<PathBuf>,
}

/// Fatal failures before the interactive loop can be established.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("the terminal is not available; run this client from an interactive terminal")]
    NotATerminal,
    #[error("terminal error: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Config(#[from] ConfigError),
}

/// Messages produced by background work.
enum Event {
    AuthStatus {
        generation: u64,
        message: String,
    },
    AuthReady {
        generation: u64,
        access_token: String,
        refresh_token: String,
        expires_at: i64,
        persist: bool,
        profile: Profile,
        service: ApiService,
        cached_count: usize,
    },
    AuthFailed {
        generation: u64,
        message: String,
    },
    DeviceCode {
        generation: u64,
        code: String,
        url: String,
    },
    LibraryLoaded {
        request: u64,
        result: Result<crate::model::LibraryData, String>,
    },
    CollectionLoaded {
        request_id: u64,
        result: Result<LoadedCollection, String>,
    },
    SearchLoaded {
        request_id: u64,
        result: Result<crate::model::SearchResults, String>,
    },
    WaveLoaded {
        request_id: u64,
        result: Result<WaveBatch, String>,
    },
    LyricsLoaded {
        request_id: u64,
        result: Result<Option<crate::lyrics::Lyrics>, String>,
    },
    CoverLoaded {
        request_id: u64,
        result: Result<crate::cover::CoverImage, String>,
    },
    DownloadFinished {
        track: crate::model::Track,
        result: Result<PathBuf, String>,
    },
    LikeFinished {
        track: crate::model::Track,
        liked: bool,
        result: Result<(), String>,
    },
    TrackReady {
        sequence: u64,
        track: crate::model::Track,
        result: Result<PathBuf, String>,
    },
}

/// Credentials and metadata for one connection attempt.
struct BootstrapRequest {
    generation: u64,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    persist: bool,
}

/// Runtime half of the application: owns services, the audio thread, and the
/// interface state.
struct Runner {
    app: App,
    config: Config,
    config_path: PathBuf,
    cache_dir: PathBuf,
    tx: UnboundedSender<Event>,
    service: Option<ApiService>,
    player: Option<AudioPlayer>,
    player_unavailable: bool,
    playback_sequence: u64,
    quit: bool,
    demo: bool,
    offline_cache: bool,
}

impl Runner {
    fn new(
        app: App,
        config: Config,
        config_path: PathBuf,
        cache_dir: PathBuf,
        offline_cache: bool,
        tx: UnboundedSender<Event>,
    ) -> Self {
        let demo = app.mode == Mode::Demo;
        Self {
            app,
            config,
            config_path,
            cache_dir,
            tx,
            service: None,
            player: None,
            player_unavailable: false,
            playback_sequence: 0,
            quit: false,
            demo,
            offline_cache,
        }
    }

    fn start_saved_session(&mut self, cli_token: Option<&str>) {
        let env_token = Config::token_from_env();
        let (access, refresh, expires, persist) =
            if let Some(token) = cli_token.map(str::trim).filter(|token| !token.is_empty()) {
                (token.to_owned(), String::new(), 0, false)
            } else if let Some(token) = env_token {
                (token, String::new(), 0, false)
            } else if !self.config.auth.access_token.is_empty()
                || !self.config.auth.refresh_token.is_empty()
            {
                (
                    self.config.auth.access_token.clone(),
                    self.config.auth.refresh_token.clone(),
                    self.config.auth.expires_at,
                    true,
                )
            } else {
                self.app.auth_note =
                    "No saved session yet. Choose how you would like to sign in.".into();
                return;
            };

        self.app.begin_connecting(if persist {
            "Restoring your session…"
        } else {
            "Connecting to Yandex Music…"
        });
        self.spawn_bootstrap(BootstrapRequest {
            generation: self.app.auth_generation,
            access_token: access,
            refresh_token: refresh,
            expires_at: expires,
            persist,
        });
    }

    fn start_device_login(&mut self, generation: u64) {
        let tx = self.tx.clone();
        let cache_dir = self.cache_dir.clone();
        let quality = self.config.playback.quality;
        tokio::spawn(async move {
            let client = match yamuse::Client::anonymous() {
                Ok(client) => client,
                Err(err) => {
                    send_auth_failure(
                        &tx,
                        generation,
                        format!("Could not start the device sign-in flow: {err}"),
                    );
                    return;
                }
            };
            let options = yamuse::DeviceAuthOptions {
                device_name: "yandex-music-tui".into(),
                ..yamuse::DeviceAuthOptions::default()
            };
            let callback_tx = tx.clone();
            let token = match client
                .device_auth(options, move |code| {
                    let _ = callback_tx.send(Event::DeviceCode {
                        generation,
                        code: code.user_code.clone().unwrap_or_else(|| "—".into()),
                        url: code
                            .verification_url
                            .clone()
                            .unwrap_or_else(|| "https://ya.ru/device".into()),
                    });
                })
                .await
            {
                Ok(token) => token,
                Err(err) => {
                    send_auth_failure(
                        &tx,
                        generation,
                        format!("Device sign-in did not complete: {err}"),
                    );
                    return;
                }
            };

            let Some(access_token) = token.access_token else {
                send_auth_failure(&tx, generation, "The device flow returned no access token.");
                return;
            };
            let refresh_token = token.refresh_token.unwrap_or_default();
            let expires_at = token
                .expires_in
                .map(|seconds| unix_now() + seconds.min(i64::MAX as u64) as i64)
                .unwrap_or(0);
            let event = bootstrap_session(
                BootstrapRequest {
                    generation,
                    access_token,
                    refresh_token,
                    expires_at,
                    persist: true,
                },
                cache_dir,
                quality,
                tx.clone(),
            )
            .await;
            let _ = tx.send(event);
        });
    }

    fn spawn_bootstrap(&self, request: BootstrapRequest) {
        let tx = self.tx.clone();
        let cache_dir = self.cache_dir.clone();
        let quality = self.config.playback.quality;
        tokio::spawn(async move {
            let event = bootstrap_session(request, cache_dir, quality, tx.clone()).await;
            let _ = tx.send(event);
        });
    }

    fn dispatch_all(&mut self, actions: Vec<Action>) {
        for action in actions {
            self.dispatch(action);
        }
    }

    fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::LoadLibrary { request } => self.load_library(request),
            Action::LoadCollection {
                request,
                request_id,
            } => self.load_collection(request, request_id),
            Action::Search { query, request_id } => self.search(query, request_id),
            Action::LoadWave {
                station,
                queue,
                request_id,
            } => self.load_wave(station, queue, request_id),
            Action::WaveFeedback {
                station,
                kind,
                track_id,
                batch_id,
                total_played_seconds,
            } => self.wave_feedback(station, kind, track_id, batch_id, total_played_seconds),
            Action::Playback { track } => self.begin_playback(track),
            Action::Pause => {
                if let Some(player) = &self.player {
                    let _ = player.send(PlayerCommand::Pause);
                }
            }
            Action::Resume => {
                if let Some(player) = &self.player {
                    let _ = player.send(PlayerCommand::Resume);
                }
            }
            Action::Seek(position) => {
                if let Some(player) = &self.player {
                    let _ = player.send(PlayerCommand::Seek(position));
                }
            }
            Action::SetVolume(volume) => {
                self.config.playback.volume = volume.clamp(0.0, 1.0);
                if let Some(player) = &self.player {
                    let _ = player.send(PlayerCommand::Volume(volume));
                }
            }
            Action::SetLike { track, liked } => self.set_like(track, liked),
            Action::Download { track } => self.download(track),
            Action::FetchCover { url, request_id } => self.fetch_cover(url, request_id),
            Action::FetchLyrics { track, request_id } => self.fetch_lyrics(track, request_id),
            Action::BeginDeviceAuth { generation } => {
                if !self.demo {
                    self.start_device_login(generation);
                }
            }
            Action::ConnectToken { token, generation } => {
                if !self.demo {
                    self.app.set_auth_status("Checking the token with Yandex…");
                    self.spawn_bootstrap(BootstrapRequest {
                        generation,
                        access_token: token,
                        refresh_token: String::new(),
                        expires_at: 0,
                        persist: true,
                    });
                }
            }
        }
    }

    fn service(&mut self) -> Option<ApiService> {
        if let Some(service) = &self.service {
            return Some(service.clone());
        }
        self.app.set_notice(
            "Sign in before using online features.",
            crate::app::NoticeKind::Info,
        );
        None
    }

    fn load_library(&mut self, request: u64) {
        if self.demo {
            let _ = self.tx.send(Event::LibraryLoaded {
                request,
                result: Ok(demo::library()),
            });
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.load_library().await;
            let data = result;
            let _ = tx.send(Event::LibraryLoaded {
                request,
                result: Ok(data),
            });
        });
    }

    fn load_collection(&mut self, request: CollectionRequest, request_id: u64) {
        if self.demo {
            let _ = self.tx.send(Event::CollectionLoaded {
                request_id,
                result: Ok(demo_collection(&request)),
            });
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = match &request {
                CollectionRequest::Playlist(playlist) => service.load_playlist(playlist).await,
                CollectionRequest::Artist(artist) => service.load_artist(artist).await,
                CollectionRequest::Album(album) => service.load_album(album).await,
            };
            let _ = tx.send(Event::CollectionLoaded { request_id, result });
        });
    }

    fn search(&mut self, query: String, request_id: u64) {
        if self.demo {
            let _ = self.tx.send(Event::SearchLoaded {
                request_id,
                result: Ok(demo::search(&query)),
            });
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.search(&query).await;
            let _ = tx.send(Event::SearchLoaded { request_id, result });
        });
    }

    fn load_wave(&mut self, station: String, queue: Option<i64>, request_id: u64) {
        if self.demo {
            let _ = self.tx.send(Event::WaveLoaded {
                request_id,
                result: Ok(demo::wave_batch(queue)),
            });
            return;
        }
        let Some(service) = self.service() else {
            let _ = self.tx.send(Event::WaveLoaded {
                request_id,
                result: Err("Your session is not connected.".into()),
            });
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.load_wave(&station, queue).await;
            let _ = tx.send(Event::WaveLoaded { request_id, result });
        });
    }

    fn wave_feedback(
        &mut self,
        station: String,
        kind: WaveFeedbackKind,
        track_id: Option<i64>,
        batch_id: Option<String>,
        total_played_seconds: Option<f64>,
    ) {
        if self.demo {
            return;
        }
        let Some(service) = &self.service else {
            return;
        };
        let service = service.clone();
        tokio::spawn(async move {
            // Rotor feedback is best-effort. Playback and UI state must never
            // wait for it or fail because the station was unreachable.
            let _ = service
                .wave_feedback(&station, kind, track_id, batch_id, total_played_seconds)
                .await;
        });
    }

    fn fetch_cover(&mut self, url: String, request_id: u64) {
        if self.demo {
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.load_cover(&url).await;
            let _ = tx.send(Event::CoverLoaded { request_id, result });
        });
    }

    fn fetch_lyrics(&mut self, track: crate::model::Track, request_id: u64) {
        if self.demo {
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.load_lyrics(&track.id).await;
            let _ = tx.send(Event::LyricsLoaded { request_id, result });
        });
    }

    fn set_like(&mut self, track: crate::model::Track, liked: bool) {
        if self.demo {
            self.app.like_succeeded(liked);
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.set_track_liked(&track.id, liked).await;
            let _ = tx.send(Event::LikeFinished {
                track,
                liked,
                result,
            });
        });
    }

    fn download(&mut self, track: crate::model::Track) {
        if self.demo {
            self.app.mark_cached(&track.id);
            return;
        }
        if !self.offline_cache {
            self.app.set_notice(
                "Offline caching is disabled in config.toml.",
                crate::app::NoticeKind::Info,
            );
            return;
        }
        let Some(service) = self.service() else {
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.ensure_cached(&track).await;
            let _ = tx.send(Event::DownloadFinished { track, result });
        });
    }

    fn begin_playback(&mut self, track: crate::model::Track) {
        if self.demo {
            return;
        }
        self.playback_sequence = self.playback_sequence.wrapping_add(1);
        let sequence = self.playback_sequence;
        if let Some(player) = &self.player {
            let _ = player.send(PlayerCommand::Stop);
        }
        if self.player_unavailable || self.player.is_none() {
            self.app.playback_failed(
                &track.id,
                "Audio output is unavailable. Is a sound server running?".into(),
            );
            return;
        }
        let Some(service) = self.service() else {
            self.app
                .playback_failed(&track.id, "Your session is not connected.".into());
            return;
        };
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = service.ensure_cached(&track).await;
            let _ = tx.send(Event::TrackReady {
                sequence,
                track,
                result,
            });
        });
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::AuthStatus {
                generation,
                message,
            } => {
                if generation == self.app.auth_generation && !self.app.signed_in {
                    self.app.set_auth_status(message);
                }
            }
            Event::AuthReady {
                generation,
                access_token,
                refresh_token,
                expires_at,
                persist,
                profile,
                service,
                cached_count,
            } => {
                if generation != self.app.auth_generation || self.app.signed_in {
                    return;
                }
                self.app.on_signed_in(profile);
                self.app.cached_count = cached_count;
                self.service = Some(service);
                if persist {
                    self.config.auth.access_token = access_token;
                    self.config.auth.refresh_token = refresh_token;
                    self.config.auth.expires_at = expires_at;
                    if let Err(err) = self.config.save(&self.config_path) {
                        self.app.set_notice(
                            format!("Signed in, but could not save the session: {err}"),
                            crate::app::NoticeKind::Error,
                        );
                    }
                }
                if !self.demo && self.player.is_none() {
                    self.player = Some(AudioPlayer::spawn(self.app.volume));
                }
                let action = self.app.begin_library_load();
                self.dispatch(action);
            }
            Event::AuthFailed {
                generation,
                message,
            } => {
                if generation == self.app.auth_generation && !self.app.signed_in {
                    self.app.auth_failed(message);
                }
            }
            Event::DeviceCode {
                generation,
                code,
                url,
            } => {
                self.app.accept_device_prompt(generation, code, url);
            }
            Event::LibraryLoaded { request, result } => match result {
                Ok(data) => self.app.library_loaded(request, data),
                Err(message) => self.app.library_failed(request, message),
            },
            Event::CollectionLoaded { request_id, result } => match result {
                Ok(data) => self.app.collection_loaded(request_id, data),
                Err(message) => self.app.collection_failed(request_id, message),
            },
            Event::SearchLoaded { request_id, result } => match result {
                Ok(results) => self.app.apply_search(request_id, results),
                Err(message) => self.app.search_failed(request_id, message),
            },
            Event::WaveLoaded { request_id, result } => match result {
                Ok(batch) => {
                    let actions = self.app.wave_loaded(request_id, batch);
                    self.dispatch_all(actions);
                }
                Err(message) => self.app.wave_failed(request_id, message),
            },
            Event::LyricsLoaded { request_id, result } => match result {
                Ok(lyrics) => self.app.lyrics_loaded(request_id, lyrics),
                Err(message) => self.app.lyrics_failed(request_id, message),
            },
            Event::CoverLoaded { request_id, result } => match result {
                Ok(cover) => self.app.cover_loaded(request_id, cover),
                Err(message) => self.app.cover_failed(request_id, message),
            },
            Event::DownloadFinished { track, result } => match result {
                Ok(_) => {
                    self.app.mark_cached(&track.id);
                    self.app.set_notice(
                        format!("“{}” is available offline.", track.title),
                        crate::app::NoticeKind::Success,
                    );
                }
                Err(message) => self.app.set_notice(
                    format!("Download failed: {message}"),
                    crate::app::NoticeKind::Error,
                ),
            },
            Event::LikeFinished {
                track,
                liked,
                result,
            } => match result {
                Ok(()) => self.app.like_succeeded(liked),
                Err(_) => self.app.like_failed(&track, liked),
            },
            Event::TrackReady {
                sequence,
                track,
                result,
            } => {
                if sequence != self.playback_sequence {
                    return;
                }
                match result {
                    Ok(path) => {
                        self.app.mark_cached(&track.id);
                        if let Some(player) = &self.player {
                            if !player.send(PlayerCommand::Play {
                                path,
                                track_id: track.id.clone(),
                            }) {
                                self.player_unavailable = true;
                                self.app.playback_failed(
                                    &track.id,
                                    "The audio thread stopped unexpectedly.".into(),
                                );
                            }
                        } else {
                            self.app.playback_failed(
                                &track.id,
                                "Audio output is unavailable. Is a sound server running?".into(),
                            );
                        }
                    }
                    Err(message) => self.app.playback_failed(&track.id, message),
                }
            }
        }
    }

    fn drain_player_events(&mut self) {
        let events = match &self.player {
            Some(player) => {
                let mut events = Vec::new();
                player.drain_events(|event| events.push(event));
                events
            }
            None => return,
        };
        for event in events {
            let current_id = self.app.current.as_ref().map(|track| track.id.clone());
            match event {
                PlayerEvent::Unavailable(message) => {
                    self.player_unavailable = true;
                    if let Some(id) = current_id {
                        self.app.playback_failed(&id, message.clone());
                    } else if self.app.signed_in {
                        self.app.set_notice(message, crate::app::NoticeKind::Error);
                    }
                }
                PlayerEvent::Started(track_id) => {
                    let actions = self.app.playback_started(&track_id);
                    self.dispatch_all(actions);
                }
                PlayerEvent::Progress { track_id, position } => {
                    if current_id.as_deref() == Some(track_id.as_str()) {
                        self.app.playback_position(position);
                    }
                }
                PlayerEvent::Finished(track_id) => {
                    if current_id.as_deref() == Some(track_id.as_str()) {
                        let actions = self.app.playback_finished();
                        self.dispatch_all(actions);
                    }
                }
                PlayerEvent::Error { track_id, message } => {
                    if let Some(track_id) = track_id {
                        if current_id.as_deref() == Some(track_id.as_str()) {
                            self.app.playback_failed(&track_id, message);
                        } else {
                            self.app.set_notice(message, crate::app::NoticeKind::Error);
                        }
                    } else {
                        self.app.set_notice(message, crate::app::NoticeKind::Error);
                    }
                }
            }
        }
    }

    fn save_config_on_exit(&mut self) {
        self.config.playback.volume = self.app.volume;
        if let Err(err) = self.config.save(&self.config_path) {
            eprintln!("warning: could not save the configuration: {err}");
        }
    }
}

async fn bootstrap_session(
    request: BootstrapRequest,
    cache_dir: PathBuf,
    quality: AudioQuality,
    tx: UnboundedSender<Event>,
) -> Event {
    let access_token = request.access_token;
    let refresh_token = request.refresh_token;
    let expires_at = request.expires_at;

    for attempt in 0..=4_u32 {
        match auth::connect(access_token.clone(), refresh_token.clone(), expires_at).await {
            Ok(session) => {
                let cache = match AudioCache::open(cache_dir.clone()) {
                    Ok(cache) => cache,
                    Err(err) => {
                        return Event::AuthFailed {
                            generation: request.generation,
                            message: format!(
                                "Could not open the offline audio cache: {err}. \
                                 Check the permissions of {}.",
                                cache_dir.display()
                            ),
                        };
                    }
                };
                let cached_count = cache.len();
                let service = match ApiService::new(session.client, cache, quality) {
                    Ok(service) => service,
                    Err(message) => {
                        return Event::AuthFailed {
                            generation: request.generation,
                            message,
                        };
                    }
                };
                return Event::AuthReady {
                    generation: request.generation,
                    access_token: session.access_token,
                    refresh_token: session.refresh_token,
                    expires_at: session.expires_at,
                    persist: request.persist,
                    profile: session.profile,
                    service,
                    cached_count,
                };
            }
            Err(failure) => {
                if failure.retryable && attempt < 4 {
                    let wait = 2_u64.pow(attempt + 1);
                    let _ = tx.send(Event::AuthStatus {
                        generation: request.generation,
                        message: format!(
                            "{} Retrying in {wait}s (attempt {}/5)…",
                            failure.message.trim_end_matches([' ', '.']),
                            attempt + 1
                        ),
                    });
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                return Event::AuthFailed {
                    generation: request.generation,
                    message: failure.message,
                };
            }
        }
    }

    Event::AuthFailed {
        generation: request.generation,
        message: "Could not connect to Yandex Music.".into(),
    }
}

fn send_auth_failure(tx: &UnboundedSender<Event>, generation: u64, message: impl Into<String>) {
    let _ = tx.send(Event::AuthFailed {
        generation,
        message: message.into(),
    });
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn demo_collection(request: &CollectionRequest) -> LoadedCollection {
    let all = demo::tracks();
    let mut tracks: Vec<_> = match request {
        CollectionRequest::Playlist(playlist) => {
            let start = (playlist.kind.unsigned_abs() as usize) % all.len().max(1);
            all.iter().cycle().skip(start).take(8).cloned().collect()
        }
        CollectionRequest::Artist(artist) => all
            .into_iter()
            .filter(|track| track.artist == artist.name)
            .collect(),
        CollectionRequest::Album(album) => all
            .into_iter()
            .filter(|track| track.album.as_deref() == Some(album.title.as_str()))
            .collect(),
    };
    if tracks.is_empty() {
        tracks = demo::tracks().into_iter().take(6).collect();
    }

    let (title, subtitle, description) = match request {
        CollectionRequest::Playlist(playlist) => (
            playlist.title.clone(),
            playlist.owner.clone(),
            Some("A hand-picked demo mix for quiet nights and long walks.".to_owned()),
        ),
        CollectionRequest::Artist(artist) => (
            artist.name.clone(),
            "Artist".to_owned(),
            Some("Popular tracks from this fictional artist.".to_owned()),
        ),
        CollectionRequest::Album(album) => (
            album.title.clone(),
            album.artist.clone(),
            Some("A complete demo album, presented with original artwork.".to_owned()),
        ),
    };

    LoadedCollection {
        title,
        subtitle,
        description,
        cover_url: None,
        tracks,
    }
}

struct TerminalUi {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalUi {
    fn enter() -> Result<Self, RuntimeError> {
        if !io::stdout().is_terminal() {
            return Err(RuntimeError::NotATerminal);
        }
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, EnterAlternateScreen, EnableBracketedPaste, Hide) {
            let _ = disable_raw_mode();
            return Err(err.into());
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        install_panic_hook();
        Ok(Self { terminal })
    }

    fn draw(&mut self, app: &App) -> Result<(), RuntimeError> {
        self.terminal.draw(|frame| crate::ui::draw(frame, app))?;
        Ok(())
    }
}

impl Drop for TerminalUi {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn install_panic_hook() {
    let original = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original(info);
    }));
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, DisableBracketedPaste, LeaveAlternateScreen);
    let _ = io::Write::flush(&mut stdout);
}

/// Run the interactive client until the user quits.
pub async fn run(options: RunOptions) -> Result<(), RuntimeError> {
    let paths = Paths::discover()?;
    let config_path = options
        .config_path
        .clone()
        .unwrap_or_else(|| paths.config_file.clone());
    let mut config = Config::load(&config_path)?;
    config.sanitise();

    let app = if options.demo {
        App::demo(&config)
    } else {
        App::new(&config, Mode::Live)
    };
    // Playback always needs a local buffer. With offline caching disabled, use a
    // temporary session directory and remove it on exit; `d` explains that
    // pinned offline copies are switched off rather than silently persisting.
    let offline_cache = config.cache.enabled;
    let cache_dir = if offline_cache {
        paths.audio_dir_for(&config)
    } else {
        paths
            .cache_dir
            .join(format!("session-{}", std::process::id()))
    };

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut runner = Runner::new(
        app,
        config,
        config_path,
        cache_dir,
        offline_cache,
        tx.clone(),
    );

    let mut terminal = TerminalUi::enter()?;

    if options.demo {
        // The demo state is complete on construction; no startup network work.
    } else if options.login {
        runner.app.auth_note =
            "Sign in to Yandex Music. Nothing is sent until you authorise this device.".into();
        let actions = runner
            .app
            .on_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        runner.dispatch_all(actions);
    } else {
        runner.start_saved_session(options.token.as_deref());
    }

    let mut terminal_events = EventStream::new();
    let mut ticker = interval(Duration::from_millis(80));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_tick = Instant::now();

    let loop_result: Result<(), RuntimeError> = loop {
        if let Err(err) = terminal.draw(&runner.app) {
            break Err(err);
        }

        tokio::select! {
            terminal_event = terminal_events.next() => {
                match terminal_event {
                    Some(Ok(TerminalEvent::Key(key))) => {
                        let actions = runner.app.on_key(key);
                        runner.dispatch_all(actions);
                    }
                    Some(Ok(TerminalEvent::Paste(text))) => {
                        runner.app.on_paste(&text);
                    }
                    Some(Ok(TerminalEvent::Resize(_, _))) => {}
                    Some(Ok(_)) => {}
                    Some(Err(err)) => break Err(RuntimeError::Io(err)),
                    None => break Ok(()),
                }
            }
            message = rx.recv() => {
                match message {
                    Some(event) => runner.handle_event(event),
                    None => break Ok(()),
                }
            }
            _ = ticker.tick() => {
                let now = Instant::now();
                let delta = now.saturating_duration_since(last_tick);
                last_tick = now;
                let actions = runner.app.tick(delta);
                runner.dispatch_all(actions);
                runner.drain_player_events();
            }
        }

        if runner.quit {
            break Ok(());
        }
    };

    runner.drain_player_events();
    if let Some(mut player) = runner.player.take() {
        player.shutdown();
    }
    runner.save_config_on_exit();
    if !offline_cache {
        let _ = std::fs::remove_dir_all(&runner.cache_dir);
    }
    drop(terminal);
    loop_result
}
