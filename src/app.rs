//! Application state, navigation, input mapping, and retry scheduling.
//!
//! The state machine is deliberately free of direct network or audio access:
//! interactions produce [`Action`] values which the runtime executes. This keeps
//! the interface deterministic and makes the whole TUI easy to render in a test
//! backend or screenshot generator.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::api::demo;
use crate::config::Config;
use crate::cover::CoverImage;
use crate::lyrics::Lyrics;
use crate::model::{
    AlbumSummary, ArtistSummary, LibraryData, LoadedCollection, PlaylistSummary, Profile,
    SearchResults, Track,
};

/// Which catalogue source is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Live,
    Demo,
}

impl Mode {
    /// Short label for the header.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "LIVE",
            Self::Demo => "DEMO",
        }
    }
}

/// Top-level library sections shown in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    Liked,
    Playlists,
    Artists,
    Albums,
}

impl Section {
    /// Sections in sidebar order.
    pub const ALL: [Self; 5] = [
        Self::Home,
        Self::Liked,
        Self::Playlists,
        Self::Artists,
        Self::Albums,
    ];

    /// Stable zero-based index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::Liked => 1,
            Self::Playlists => 2,
            Self::Artists => 3,
            Self::Albums => 4,
        }
    }

    /// Full title used in content headings.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Liked => "Liked tracks",
            Self::Playlists => "Playlists",
            Self::Artists => "Artists",
            Self::Albums => "Albums",
        }
    }

    /// Muted uppercase eyebrow above the title.
    #[must_use]
    pub const fn eyebrow(self) -> &'static str {
        match self {
            Self::Home => "YOUR LIBRARY",
            Self::Liked => "COLLECTION",
            Self::Playlists => "SAVED & CREATED",
            Self::Artists => "PEOPLE YOU FOLLOW",
            Self::Albums => "SAVED RECORDS",
        }
    }

    /// Nerd Font icon used by the sidebar.
    #[must_use]
    pub const fn icon(self) -> &'static str {
        // Font Awesome codepoints embedded in Nerd Fonts.
        match self {
            Self::Home => "\u{f015}",
            Self::Liked => "\u{f004}",
            Self::Playlists => "\u{f03a}",
            Self::Artists => "\u{f007}",
            Self::Albums => "\u{f001}",
        }
    }

    /// Numeric shortcut.
    #[must_use]
    pub const fn shortcut(self) -> char {
        match self {
            Self::Home => '1',
            Self::Liked => '2',
            Self::Playlists => '3',
            Self::Artists => '4',
            Self::Albums => '5',
        }
    }

    /// Resolve a numeric shortcut.
    #[must_use]
    pub fn from_shortcut(ch: char) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|section| section.shortcut() == ch)
    }
}

/// Keyboard focus between the three main panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
    Inspector,
}

impl Focus {
    /// Next pane in Tab order.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Sidebar => Self::Main,
            Self::Main => Self::Inspector,
            Self::Inspector => Self::Sidebar,
        }
    }

    /// Previous pane in Shift+Tab order.
    #[must_use]
    pub const fn previous(self) -> Self {
        match self {
            Self::Sidebar => Self::Inspector,
            Self::Main => Self::Sidebar,
            Self::Inspector => Self::Main,
        }
    }
}

/// Right-hand pane tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Queue,
    Lyrics,
}

/// A request for a detailed collection.
#[derive(Debug, Clone)]
pub enum CollectionRequest {
    Playlist(PlaylistSummary),
    Artist(ArtistSummary),
    Album(AlbumSummary),
}

impl CollectionRequest {
    /// Loading title while the request is in flight.
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::Playlist(playlist) => playlist.title.clone(),
            Self::Artist(artist) => artist.name.clone(),
            Self::Album(album) => album.title.clone(),
        }
    }

    /// Small category label.
    #[must_use]
    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Playlist(_) => "PLAYLIST",
            Self::Artist(_) => "ARTIST",
            Self::Album(_) => "ALBUM",
        }
    }

    /// Whether two requests point at the same collection.
    #[must_use]
    pub fn same_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Playlist(left), Self::Playlist(right)) => left.key() == right.key(),
            (Self::Artist(left), Self::Artist(right)) => left.id == right.id,
            (Self::Album(left), Self::Album(right)) => left.id == right.id,
            _ => false,
        }
    }
}

/// The currently displayed middle-pane content.
#[derive(Debug, Clone)]
pub enum View {
    Library(Section),
    Collection(Box<CollectionView>),
}

/// A detailed playlist, artist, or album view.
#[derive(Debug, Clone)]
pub struct CollectionView {
    pub request: CollectionRequest,
    pub request_id: u64,
    pub data: Option<LoadedCollection>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Search input focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFocus {
    Input,
    Results,
}

/// The state of the search overlay.
#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub focus: SearchFocus,
    pub results: SearchResults,
    pub selected: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub request_id: u64,
    pub requested_query: String,
    pub edited_at: Instant,
}

/// Search result categories in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Track,
    Album,
    Artist,
    Playlist,
}

/// A flat search hit, used by both navigation and rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchHit {
    pub kind: SearchKind,
    pub index: usize,
}

/// A modal overlay.
#[derive(Debug, Clone)]
pub enum Overlay {
    Search(SearchState),
    Help,
    TokenEntry(TokenEntry),
}

/// Manual token entry.
#[derive(Debug, Clone)]
pub struct TokenEntry {
    pub value: String,
    pub busy: bool,
    pub error: Option<String>,
}

/// The OAuth device challenge being shown to the listener.
#[derive(Debug, Clone)]
pub struct DevicePrompt {
    pub user_code: String,
    pub verification_url: String,
}

/// A transient footer message.
#[derive(Debug, Clone)]
pub struct Notice {
    pub text: String,
    pub kind: NoticeKind,
    pub until: Instant,
}

/// Colour family for a footer notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Error,
}

impl Notice {
    fn new(text: impl Into<String>, kind: NoticeKind) -> Self {
        let lifetime = match kind {
            NoticeKind::Error => Duration::from_secs(10),
            NoticeKind::Info | NoticeKind::Success => Duration::from_secs(5),
        };
        Self {
            text: text.into(),
            kind,
            until: Instant::now() + lifetime,
        }
    }
}

/// What an automatic retry should refresh.
#[derive(Debug, Clone)]
enum RetryTarget {
    Library,
    Collection(CollectionRequest),
}

#[derive(Debug, Clone)]
struct RetryPlan {
    target: RetryTarget,
    attempt: u32,
    due: Option<Instant>,
    reason: String,
}

impl RetryTarget {
    fn label(&self) -> &'static str {
        match self {
            Self::Library => "library sync",
            Self::Collection(_) => "collection",
        }
    }
}

/// Side effects emitted by the state machine.
#[derive(Debug)]
pub enum Action {
    Quit,
    LoadLibrary {
        request: u64,
    },
    LoadCollection {
        request: CollectionRequest,
        request_id: u64,
    },
    Search {
        query: String,
        request_id: u64,
    },
    Playback {
        track: Track,
    },
    Pause,
    Resume,
    Seek(Duration),
    SetVolume(f32),
    SetLike {
        track: Track,
        liked: bool,
    },
    Download {
        track: Track,
    },
    FetchCover {
        url: String,
        request_id: u64,
    },
    FetchLyrics {
        track: Track,
        request_id: u64,
    },
    BeginDeviceAuth {
        generation: u64,
    },
    ConnectToken {
        token: String,
        generation: u64,
    },
}

const SEARCH_NAV_INDEX: usize = 5;

/// The complete application state rendered by the UI.
pub struct App {
    pub mode: Mode,
    pub signed_in: bool,
    pub auth_busy: bool,
    pub auth_error: Option<String>,
    pub auth_note: String,
    pub auth_device: Option<DevicePrompt>,
    pub auth_generation: u64,

    pub profile: Profile,
    pub library: Option<LibraryData>,
    pub library_loading: bool,
    pub library_error: Option<String>,
    library_request: u64,
    collection_seq: u64,

    pub section: Section,
    pub nav_index: usize,
    pub focus: Focus,
    pub view: View,
    pub main_selected: usize,

    pub queue: Vec<Track>,
    pub queue_cursor: usize,
    pub queue_selected: usize,
    pub inspector_tab: InspectorTab,

    pub current: Option<Track>,
    pub position: Duration,
    pub duration: Duration,
    pub playing: bool,
    pub buffering: bool,
    pub volume: f32,
    pub muted: bool,
    pub cached_count: usize,
    pub cached_ids: HashSet<String>,
    pub liked_ids: HashSet<String>,

    pub lyrics: Option<Lyrics>,
    pub lyrics_loading: bool,
    pub lyrics_error: Option<String>,
    lyrics_request: u64,
    pub lyrics_offset: i32,

    pub cover: Option<CoverImage>,
    pub cover_loading: bool,
    cover_request: u64,

    pub now_playing: bool,
    pub overlay: Option<Overlay>,
    retry: Option<RetryPlan>,
    pub notice: Option<Notice>,

    pub animations: bool,
    pub show_covers: bool,
    pub compact_sidebar_below: u16,
    pub seek_step: u64,
    pub animation_tick: u64,
    pub last_track_change: Instant,
}

impl App {
    /// Create a signed-out application ready for authentication.
    #[must_use]
    pub fn new(config: &Config, mode: Mode) -> Self {
        Self {
            mode,
            signed_in: false,
            auth_busy: false,
            auth_error: None,
            auth_note: String::new(),
            auth_device: None,
            auth_generation: 0,
            profile: Profile::default(),
            library: None,
            library_loading: false,
            library_error: None,
            library_request: 0,
            collection_seq: 0,
            section: Section::Home,
            nav_index: 0,
            focus: Focus::Main,
            view: View::Library(Section::Home),
            main_selected: 0,
            queue: Vec::new(),
            queue_cursor: 0,
            queue_selected: 0,
            inspector_tab: InspectorTab::Queue,
            current: None,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            playing: false,
            buffering: false,
            volume: config.playback.volume,
            muted: false,
            cached_count: 0,
            cached_ids: HashSet::new(),
            liked_ids: HashSet::new(),
            lyrics: None,
            lyrics_loading: false,
            lyrics_error: None,
            lyrics_request: 0,
            lyrics_offset: 0,
            cover: None,
            cover_loading: false,
            cover_request: 0,
            now_playing: false,
            overlay: None,
            retry: None,
            notice: None,
            animations: config.ui.animations,
            show_covers: config.ui.cover_art,
            compact_sidebar_below: config.ui.compact_sidebar_below,
            seek_step: config.playback.seek_step_seconds,
            animation_tick: 0,
            last_track_change: Instant::now(),
        }
    }

    /// Build the offline showcase state, complete with a queue and lyrics.
    #[must_use]
    pub fn demo(config: &Config) -> Self {
        let mut app = Self::new(config, Mode::Demo);
        let library = demo::library();
        app.profile = library.profile.clone();
        app.liked_ids = library
            .liked_tracks
            .iter()
            .map(|track| track.id.clone())
            .collect();
        app.library = Some(library);
        app.signed_in = true;
        app.queue = demo::queue();
        app.queue_cursor = 0;
        app.queue_selected = 0;
        app.current = app.queue.first().cloned();
        app.duration = app.current.as_ref().map_or(Duration::ZERO, Track::duration);
        app.position = demo::start_position();
        app.playing = false;
        app.buffering = false;
        app.cached_count = 3;
        app.cached_ids
            .extend(app.queue.iter().take(3).map(|track| track.id.clone()));
        app.lyrics = Some(demo::lyrics());
        app.lyrics_loading = false;
        if app.show_covers {
            app.cover = app.current.as_ref().map(demo::cover);
        }
        app.notice = None;
        app.focus = Focus::Main;
        app
    }

    /// Show an authentication status and enter the waiting state.
    pub fn begin_connecting(&mut self, note: impl Into<String>) {
        self.signed_in = false;
        self.auth_busy = true;
        self.auth_error = None;
        self.auth_note = note.into();
        self.overlay = None;
    }

    /// Accept a successful session bootstrap.
    pub fn on_signed_in(&mut self, profile: Profile) {
        self.signed_in = true;
        self.auth_busy = false;
        self.auth_error = None;
        self.auth_device = None;
        self.auth_note.clear();
        self.overlay = None;
        self.profile = profile;
        self.section = Section::Home;
        self.nav_index = 0;
        self.view = View::Library(Section::Home);
        self.main_selected = 0;
        self.focus = Focus::Main;
        self.now_playing = false;
    }

    /// Report a failed authentication attempt.
    pub fn auth_failed(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.signed_in = false;
        self.auth_busy = false;
        self.auth_error = Some(message.clone());
        self.auth_note = "Could not sign in. Try again when you are ready.".into();
        if let Some(Overlay::TokenEntry(entry)) = &mut self.overlay {
            entry.busy = false;
            entry.error = Some(message);
        }
    }

    /// Update the transient status shown while connecting or refreshing.
    pub fn set_auth_status(&mut self, note: impl Into<String>) {
        self.auth_busy = true;
        self.auth_note = note.into();
    }

    /// Accept an OAuth challenge if it belongs to the current attempt.
    pub fn accept_device_prompt(&mut self, generation: u64, code: String, url: String) -> bool {
        if generation != self.auth_generation || self.signed_in {
            return false;
        }
        self.auth_busy = true;
        self.auth_error = None;
        self.auth_note = "Waiting for confirmation in your browser.".into();
        self.auth_device = Some(DevicePrompt {
            user_code: code,
            verification_url: url,
        });
        true
    }

    /// Accept a device-flow token if the attempt is still current.
    pub fn accept_device_token(&mut self, generation: u64) -> bool {
        if generation != self.auth_generation || self.signed_in {
            return false;
        }
        self.auth_device = None;
        self.auth_busy = true;
        self.auth_error = None;
        self.auth_note = "Token received. Opening your library…".into();
        true
    }

    /// Start loading the full library.
    pub fn begin_library_load(&mut self) -> Action {
        self.library_request = self.library_request.wrapping_add(1);
        self.library_loading = true;
        self.library_error = None;
        Action::LoadLibrary {
            request: self.library_request,
        }
    }

    /// Apply a full library result, keeping partial data on endpoint errors.
    pub fn library_loaded(&mut self, request: u64, data: LibraryData) {
        if request != self.library_request {
            return;
        }
        self.library_loading = false;
        self.library_error = None;
        let errors = data.errors.clone();
        self.liked_ids = data
            .liked_tracks
            .iter()
            .map(|track| track.id.clone())
            .collect();
        if !data.profile.display_name.is_empty() || !data.profile.login.is_empty() {
            self.profile = data.profile.clone();
        }
        self.library = Some(data);
        self.clamp_main_selection();
        if errors.is_empty() {
            if matches!(self.retry_target(), Some(RetryTarget::Library)) {
                self.retry = None;
            }
        } else {
            let reason = errors.join(" · ");
            self.schedule_retry(RetryTarget::Library, reason);
        }
    }

    /// Apply a library failure.
    pub fn library_failed(&mut self, request: u64, message: String) {
        if request != self.library_request {
            return;
        }
        self.library_loading = false;
        if self.library.is_none() {
            self.library_error = Some(message.clone());
        }
        self.schedule_retry(RetryTarget::Library, message);
    }

    /// Open a detailed playlist, artist, or album view.
    pub fn open_collection(&mut self, request: CollectionRequest) -> Action {
        self.library_error = None;
        self.main_selected = 0;
        if matches!(self.retry_target(), Some(RetryTarget::Collection(_))) {
            self.retry = None;
        }
        self.collection_seq = self.collection_seq.wrapping_add(1);
        let request_id = self.collection_seq;
        self.view = View::Collection(Box::new(CollectionView {
            request: request.clone(),
            request_id,
            data: None,
            loading: true,
            error: None,
        }));
        Action::LoadCollection {
            request,
            request_id,
        }
    }

    /// Apply a collection result if it still matches the active view.
    pub fn collection_loaded(&mut self, request_id: u64, data: LoadedCollection) {
        let request = {
            let View::Collection(collection) = &mut self.view else {
                return;
            };
            if collection.request_id != request_id {
                return;
            }
            collection.loading = false;
            collection.data = Some(data);
            collection.error = None;
            collection.request.clone()
        };
        self.main_selected = 0;
        if matches!(
            self.retry_target(),
            Some(RetryTarget::Collection(retry)) if retry.same_target(&request)
        ) {
            self.retry = None;
        }
    }

    /// Apply a collection failure if it still matches the active view.
    pub fn collection_failed(&mut self, request_id: u64, message: String) {
        let request = {
            let View::Collection(collection) = &mut self.view else {
                return;
            };
            if collection.request_id != request_id {
                return;
            }
            collection.loading = false;
            collection.error = Some(message.clone());
            collection.request.clone()
        };
        self.schedule_retry(RetryTarget::Collection(request), message);
    }

    /// Start a search for a debounced query.
    pub fn apply_search(&mut self, request_id: u64, results: SearchResults) {
        let Some(Overlay::Search(search)) = &mut self.overlay else {
            return;
        };
        if search.request_id != request_id {
            return;
        }
        search.loading = false;
        search.error = None;
        search.results = results;
        search.selected = search.selected.min(search.result_count().saturating_sub(1));
    }

    /// Surface a search failure without discarding the query.
    pub fn search_failed(&mut self, request_id: u64, message: String) {
        let Some(Overlay::Search(search)) = &mut self.overlay else {
            return;
        };
        if search.request_id != request_id {
            return;
        }
        search.loading = false;
        search.error = Some(message);
    }

    /// Apply a decoded cover if it belongs to the active track.
    pub fn cover_loaded(&mut self, request: u64, cover: CoverImage) {
        if request != self.cover_request || !self.show_covers {
            return;
        }
        self.cover_loading = false;
        self.cover = Some(cover);
    }

    /// Keep the procedural cover when network art is unavailable.
    pub fn cover_failed(&mut self, request: u64, _message: String) {
        if request == self.cover_request {
            self.cover_loading = false;
        }
    }

    /// Apply lyrics for the active track.
    pub fn lyrics_loaded(&mut self, request: u64, lyrics: Option<Lyrics>) {
        if request != self.lyrics_request {
            return;
        }
        self.lyrics_loading = false;
        self.lyrics_error = None;
        self.lyrics_offset = 0;
        self.lyrics = lyrics;
    }

    /// Surface lyrics failure for the active track.
    pub fn lyrics_failed(&mut self, request: u64, message: String) {
        if request != self.lyrics_request {
            return;
        }
        self.lyrics_loading = false;
        self.lyrics_error = Some(message);
    }

    /// Mark playback as ready.
    pub fn playback_started(&mut self, track_id: &str) {
        if self.current.as_ref().map(|track| track.id.as_str()) != Some(track_id) {
            return;
        }
        self.buffering = false;
        self.playing = true;
        self.last_track_change = Instant::now();
    }

    /// Mark playback as failed while keeping the selection intact.
    pub fn playback_failed(&mut self, track_id: &str, message: String) {
        if self.current.as_ref().map(|track| track.id.as_str()) != Some(track_id) {
            return;
        }
        self.buffering = false;
        self.playing = false;
        self.set_notice(message, NoticeKind::Error);
    }

    /// Update the current playback position.
    pub fn playback_position(&mut self, position: Duration) {
        if self.current.is_some() {
            self.position = position.min(self.duration.max(position));
        }
    }

    /// Handle the end of a track. Returns playback actions for the next track.
    pub fn playback_finished(&mut self) -> Vec<Action> {
        if self.queue.len() <= 1 {
            self.playing = false;
            self.position = self.duration;
            return Vec::new();
        }
        if self.queue_cursor + 1 < self.queue.len() {
            self.start_queue_index(self.queue_cursor + 1)
        } else {
            self.playing = false;
            self.position = self.duration;
            Vec::new()
        }
    }

    /// Record that a track is available offline.
    pub fn mark_cached(&mut self, track_id: &str) {
        self.cached_ids.insert(track_id.to_owned());
        self.cached_count = self.cached_ids.len().max(self.cached_count);
    }

    /// Revert an optimistic like after the API rejects it.
    pub fn like_failed(&mut self, track: &Track, attempted_liked: bool) {
        if attempted_liked {
            self.liked_ids.remove(&track.id);
            if let Some(library) = &mut self.library {
                library.liked_tracks.retain(|saved| saved.id != track.id);
            }
        } else {
            self.liked_ids.insert(track.id.clone());
            if let Some(library) = &mut self.library {
                if !library
                    .liked_tracks
                    .iter()
                    .any(|saved| saved.id == track.id)
                {
                    library.liked_tracks.push(track.clone());
                }
            }
        }
        self.set_notice("Could not update your liked tracks.", NoticeKind::Error);
        self.clamp_main_selection();
    }

    /// Confirm a server-side like change.
    pub fn like_succeeded(&mut self, liked: bool) {
        self.set_notice(
            if liked {
                "Added to liked tracks."
            } else {
                "Removed from liked tracks."
            },
            NoticeKind::Success,
        );
    }

    /// Show a transient message in the footer.
    pub fn set_notice(&mut self, message: impl Into<String>, kind: NoticeKind) {
        self.notice = Some(Notice::new(message, kind));
    }

    /// The notice that has not yet expired.
    #[must_use]
    pub fn visible_notice(&self) -> Option<&Notice> {
        self.notice
            .as_ref()
            .filter(|notice| notice.until >= Instant::now())
    }

    /// A short automatic-retry description for the footer.
    #[must_use]
    pub fn retry_status(&self) -> Option<String> {
        let plan = self.retry.as_ref()?;
        let target = plan.target.label();
        match plan.due {
            Some(due) => {
                let seconds = due
                    .saturating_duration_since(Instant::now())
                    .as_secs_f32()
                    .ceil() as u64;
                Some(format!("{target}: retry {}/5 in {seconds}s", plan.attempt))
            }
            None => Some(format!("{target}: retrying…")),
        }
    }

    /// One-line cause for the retry UI.
    #[must_use]
    pub fn retry_reason(&self) -> Option<&str> {
        self.retry.as_ref().map(|plan| plan.reason.as_str())
    }

    /// Whether the app has an unresolved error worth showing in the content pane.
    #[must_use]
    pub fn content_error(&self) -> Option<&str> {
        match &self.view {
            View::Library(Section::Home | Section::Liked) => self.library_error.as_deref(),
            View::Library(_) => self.library_error.as_deref(),
            View::Collection(collection) => collection.error.as_deref(),
        }
    }

    /// Start a full-screen now-playing view.
    pub fn open_now_playing(&mut self) {
        if self.current.is_some() {
            self.now_playing = true;
        } else {
            self.set_notice("Play a track first.", NoticeKind::Info);
        }
    }

    /// Number of items selected by j/k in the current middle pane.
    #[must_use]
    pub fn main_len(&self) -> usize {
        match &self.view {
            View::Collection(collection) => {
                collection.data.as_ref().map_or(0, |data| data.tracks.len())
            }
            View::Library(section) => match section {
                Section::Home | Section::Liked => self
                    .library
                    .as_ref()
                    .map_or(0, |library| library.liked_tracks.len()),
                Section::Playlists => self
                    .library
                    .as_ref()
                    .map_or(0, |library| library.playlists.len()),
                Section::Artists => self
                    .library
                    .as_ref()
                    .map_or(0, |library| library.artists.len()),
                Section::Albums => self
                    .library
                    .as_ref()
                    .map_or(0, |library| library.albums.len()),
            },
        }
    }

    /// Tracks associated with the current middle pane, when it contains tracks.
    #[must_use]
    pub fn visible_tracks(&self) -> &[Track] {
        match &self.view {
            View::Collection(collection) => collection
                .data
                .as_ref()
                .map_or(&[], |data| data.tracks.as_slice()),
            View::Library(Section::Home | Section::Liked) => self
                .library
                .as_ref()
                .map_or(&[], |library| library.liked_tracks.as_slice()),
            View::Library(_) => &[],
        }
    }

    fn clamp_main_selection(&mut self) {
        self.main_selected = self.main_selected.min(self.main_len().saturating_sub(1));
    }

    fn retry_target(&self) -> Option<&RetryTarget> {
        self.retry.as_ref().map(|plan| &plan.target)
    }

    fn schedule_retry(&mut self, target: RetryTarget, reason: String) {
        let previous_attempt = self.retry.as_ref().map_or(0, |plan| plan.attempt);
        let attempt = (previous_attempt + 1).min(5);
        let delay_seconds = 2_u64.pow(attempt).min(60);
        let due = Instant::now() + Duration::from_secs(delay_seconds);
        self.retry = Some(RetryPlan {
            target,
            attempt,
            due: Some(due),
            reason,
        });
    }

    /// Advance animations and drive debounce/retry/demo playback. Returns side
    /// effects that should be executed by the runtime.
    pub fn tick(&mut self, delta: Duration) -> Vec<Action> {
        self.animation_tick = self.animation_tick.wrapping_add(1);
        let mut actions = Vec::new();

        if self.mode == Mode::Demo && self.playing && !self.buffering {
            self.position = self.position.saturating_add(delta);
            if self.duration > Duration::ZERO && self.position >= self.duration {
                self.position = self.duration;
                actions.extend(self.playback_finished());
            }
        }

        // Debounced catalogue search.
        let should_search = match &self.overlay {
            Some(Overlay::Search(search)) => {
                let query = search.query.trim();
                !query.is_empty()
                    && query != search.requested_query
                    && search.edited_at.elapsed() >= Duration::from_millis(320)
            }
            _ => false,
        };
        if should_search {
            if let Some(Overlay::Search(search)) = &mut self.overlay {
                search.requested_query = search.query.trim().to_owned();
                search.request_id = search.request_id.wrapping_add(1);
                search.loading = true;
                search.error = None;
                actions.push(Action::Search {
                    query: search.requested_query.clone(),
                    request_id: search.request_id,
                });
            }
        }

        // Automatic exponential-backoff refresh.
        let retry_due = self
            .retry
            .as_ref()
            .and_then(|plan| plan.due)
            .is_some_and(|due| due <= Instant::now());
        if retry_due {
            let target = self.retry.as_mut().map(|plan| {
                plan.due = None;
                plan.target.clone()
            });
            match target {
                Some(RetryTarget::Library) => {
                    actions.push(self.begin_library_load());
                }
                Some(RetryTarget::Collection(request)) => {
                    let matches_view = matches!(
                        &self.view,
                        View::Collection(collection) if collection.request.same_target(&request)
                    );
                    if matches_view {
                        self.collection_seq = self.collection_seq.wrapping_add(1);
                        let request_id = self.collection_seq;
                        if let View::Collection(collection) = &mut self.view {
                            collection.loading = true;
                            collection.error = None;
                            collection.request_id = request_id;
                            actions.push(Action::LoadCollection {
                                request: collection.request.clone(),
                                request_id,
                            });
                        }
                    } else {
                        self.retry = None;
                    }
                }
                None => {}
            }
        }

        actions
    }

    /// Handle one terminal key event.
    pub fn on_key(&mut self, key: KeyEvent) -> Vec<Action> {
        if key.kind == KeyEventKind::Release {
            return Vec::new();
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return vec![Action::Quit];
        }

        if self.overlay.is_some() {
            return self.on_overlay_key(key);
        }

        if !self.signed_in {
            return self.on_auth_key(key);
        }

        if key.code == KeyCode::Char('?') {
            self.overlay = Some(Overlay::Help);
            return Vec::new();
        }
        if key.code == KeyCode::Char('/') {
            self.open_search();
            return Vec::new();
        }
        if key.code == KeyCode::Char('q') {
            return vec![Action::Quit];
        }

        let mut actions = Vec::new();
        match key.code {
            KeyCode::Tab => {
                self.focus = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.focus.previous()
                } else {
                    self.focus.next()
                };
            }
            KeyCode::BackTab => self.focus = self.focus.previous(),
            KeyCode::Char('v') => self.toggle_now_playing(),
            KeyCode::Esc => self.escape(),
            KeyCode::Char(' ') => actions.extend(self.toggle_playback()),
            KeyCode::Char('n') => actions.extend(self.queue_step(true, true)),
            KeyCode::Char('p') => actions.extend(self.queue_step(false, true)),
            KeyCode::Char('f') => actions.extend(self.toggle_like()),
            KeyCode::Char('d') => actions.extend(self.download_selected()),
            KeyCode::Char('m') => actions.extend(self.toggle_mute()),
            KeyCode::Char('+') | KeyCode::Char('=') => actions.extend(self.adjust_volume(0.05)),
            KeyCode::Char('-') | KeyCode::Char('_') => actions.extend(self.adjust_volume(-0.05)),
            KeyCode::Char('i') => self.toggle_inspector_tab(),
            KeyCode::Char('[') => self.inspector_tab = InspectorTab::Queue,
            KeyCode::Char(']') => self.inspector_tab = InspectorTab::Lyrics,
            KeyCode::Char('r') => self.request_manual_retry(),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
                actions.extend(self.seek_relative(-(self.seek_step as i64)));
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
                actions.extend(self.seek_relative(self.seek_step as i64));
            }
            KeyCode::Left if self.now_playing => {
                actions.extend(self.seek_relative(-(self.seek_step as i64)));
            }
            KeyCode::Right if self.now_playing => {
                actions.extend(self.seek_relative(self.seek_step as i64));
            }
            KeyCode::Char('j') if self.now_playing => {
                self.lyrics_offset = self.lyrics_offset.saturating_add(1);
            }
            KeyCode::Char('k') if self.now_playing => {
                self.lyrics_offset = self.lyrics_offset.saturating_sub(1);
            }
            KeyCode::Char('g') if self.now_playing => {
                self.lyrics_offset = 0;
            }
            KeyCode::Char(ch) if Section::from_shortcut(ch).is_some() => {
                if let Some(section) = Section::from_shortcut(ch) {
                    self.switch_section(section, false);
                }
            }
            _ if self.now_playing => {}
            _ => match self.focus {
                Focus::Sidebar => actions.extend(self.on_sidebar_key(key)),
                Focus::Main => actions.extend(self.on_main_key(key)),
                Focus::Inspector => actions.extend(self.on_inspector_key(key)),
            },
        }
        actions
    }

    /// Handle a bracketed paste.
    pub fn on_paste(&mut self, text: &str) {
        match &mut self.overlay {
            Some(Overlay::Search(search)) => {
                if search.focus == SearchFocus::Input {
                    for ch in text.chars().filter(|ch| !ch.is_control()) {
                        search.query.push(ch);
                    }
                    search.selected = 0;
                    search.edited_at = Instant::now();
                }
            }
            Some(Overlay::TokenEntry(entry)) if !entry.busy => {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    entry.value.push(ch);
                }
            }
            _ => {}
        }
    }

    fn on_auth_key(&mut self, key: KeyEvent) -> Vec<Action> {
        match key.code {
            KeyCode::Char('q') => vec![Action::Quit],
            KeyCode::Char('?') => {
                self.overlay = Some(Overlay::Help);
                Vec::new()
            }
            KeyCode::Char('d') if !self.auth_busy => {
                self.auth_generation = self.auth_generation.wrapping_add(1);
                self.auth_busy = true;
                self.auth_error = None;
                self.auth_device = None;
                self.auth_note = "Requesting a device code…".into();
                vec![Action::BeginDeviceAuth {
                    generation: self.auth_generation,
                }]
            }
            KeyCode::Char('t') if !self.auth_busy => {
                self.overlay = Some(Overlay::TokenEntry(TokenEntry {
                    value: String::new(),
                    busy: false,
                    error: None,
                }));
                Vec::new()
            }
            KeyCode::Esc if self.auth_busy || self.auth_device.is_some() => {
                self.auth_generation = self.auth_generation.wrapping_add(1);
                self.auth_busy = false;
                self.auth_device = None;
                self.auth_error = None;
                self.auth_note = "Sign-in cancelled. Press d to try again.".into();
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn on_overlay_key(&mut self, key: KeyEvent) -> Vec<Action> {
        let is_help = matches!(self.overlay, Some(Overlay::Help));
        if is_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.overlay = None;
            }
            return Vec::new();
        }

        match self.overlay.clone() {
            Some(Overlay::Search(mut search)) => {
                let mut actions = Vec::new();
                match key.code {
                    KeyCode::Esc => {
                        if search.focus == SearchFocus::Results && !search.query.is_empty() {
                            search.focus = SearchFocus::Input;
                            self.overlay = Some(Overlay::Search(search));
                        } else {
                            self.overlay = None;
                        }
                    }
                    KeyCode::Tab => {
                        search.focus = if search.focus == SearchFocus::Input {
                            SearchFocus::Results
                        } else {
                            SearchFocus::Input
                        };
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Enter => {
                        if search.focus == SearchFocus::Input && !search.results.is_empty() {
                            search.focus = SearchFocus::Results;
                            self.overlay = Some(Overlay::Search(search));
                        } else if search.focus == SearchFocus::Results {
                            self.overlay = Some(Overlay::Search(search));
                            actions.extend(self.activate_search_hit());
                        }
                    }
                    KeyCode::Backspace if search.focus == SearchFocus::Input => {
                        search.query.pop();
                        search.selected = 0;
                        search.loading = false;
                        search.error = None;
                        search.edited_at = Instant::now();
                        if search.query.trim().is_empty() {
                            search.results = SearchResults::default();
                            search.requested_query.clear();
                        }
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Char(ch)
                        if search.focus == SearchFocus::Input
                            && !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        search.query.push(ch);
                        search.selected = 0;
                        search.loading = false;
                        search.error = None;
                        search.edited_at = Instant::now();
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Down | KeyCode::Char('j') if search.focus == SearchFocus::Input => {
                        if !search.results.is_empty() {
                            search.focus = SearchFocus::Results;
                            search.selected =
                                search.selected.min(search.result_count().saturating_sub(1));
                            self.overlay = Some(Overlay::Search(search));
                        }
                    }
                    KeyCode::Up if search.focus == SearchFocus::Input => {}
                    KeyCode::Down | KeyCode::Char('j') if search.focus == SearchFocus::Results => {
                        search.selected =
                            (search.selected + 1).min(search.result_count().saturating_sub(1));
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Up | KeyCode::Char('k') if search.focus == SearchFocus::Results => {
                        search.selected = search.selected.saturating_sub(1);
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Char('g') if search.focus == SearchFocus::Results => {
                        search.selected = 0;
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Char('G') if search.focus == SearchFocus::Results => {
                        search.selected = search.result_count().saturating_sub(1);
                        self.overlay = Some(Overlay::Search(search));
                    }
                    KeyCode::Backspace | KeyCode::Char(_) => {
                        self.overlay = Some(Overlay::Search(search));
                    }
                    _ => {
                        self.overlay = Some(Overlay::Search(search));
                    }
                }
                actions
            }
            Some(Overlay::TokenEntry(mut entry)) => {
                match key.code {
                    KeyCode::Esc if !entry.busy => self.overlay = None,
                    KeyCode::Backspace if !entry.busy => {
                        entry.value.pop();
                        entry.error = None;
                        self.overlay = Some(Overlay::TokenEntry(entry));
                    }
                    KeyCode::Enter if !entry.busy => {
                        let token = entry.value.trim().to_owned();
                        if token.is_empty() {
                            entry.error = Some("Paste a token first.".into());
                            self.overlay = Some(Overlay::TokenEntry(entry));
                        } else {
                            entry.busy = true;
                            entry.error = None;
                            self.auth_busy = true;
                            self.auth_note = "Checking the token with Yandex…".into();
                            self.overlay = Some(Overlay::TokenEntry(entry));
                            self.auth_generation = self.auth_generation.wrapping_add(1);
                            return vec![Action::ConnectToken {
                                token,
                                generation: self.auth_generation,
                            }];
                        }
                    }
                    KeyCode::Char(ch)
                        if !entry.busy
                            && !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        entry.value.push(ch);
                        entry.error = None;
                        self.overlay = Some(Overlay::TokenEntry(entry));
                    }
                    _ => self.overlay = Some(Overlay::TokenEntry(entry)),
                }
                Vec::new()
            }
            Some(Overlay::Help) | None => Vec::new(),
        }
    }

    fn on_sidebar_key(&mut self, key: KeyEvent) -> Vec<Action> {
        let last_index = SEARCH_NAV_INDEX;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.nav_index = (self.nav_index + 1).min(last_index);
                self.nav_changed();
                Vec::new()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.nav_index = self.nav_index.saturating_sub(1);
                self.nav_changed();
                Vec::new()
            }
            KeyCode::Char('g') => {
                self.nav_index = 0;
                self.nav_changed();
                Vec::new()
            }
            KeyCode::Char('G') => {
                self.nav_index = last_index;
                self.nav_changed();
                Vec::new()
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                if self.nav_index == SEARCH_NAV_INDEX {
                    self.open_search();
                    Vec::new()
                } else {
                    self.focus = Focus::Main;
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    fn nav_changed(&mut self) {
        if let Some(section) = Section::ALL.get(self.nav_index).copied() {
            self.section = section;
            self.view = View::Library(section);
            self.main_selected = 0;
        }
    }

    fn on_main_key(&mut self, key: KeyEvent) -> Vec<Action> {
        let length = self.main_len();
        let mut actions = Vec::new();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if length > 0 {
                    self.main_selected = (self.main_selected + 1).min(length - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.main_selected = self.main_selected.saturating_sub(1);
            }
            KeyCode::Char('g') => self.main_selected = 0,
            KeyCode::Char('G') => self.main_selected = length.saturating_sub(1),
            KeyCode::Char('h') | KeyCode::Left => self.focus = Focus::Sidebar,
            KeyCode::Char('l') | KeyCode::Right => self.focus = Focus::Inspector,
            KeyCode::Backspace if !matches!(self.view, View::Library(_)) => {
                self.back_to_library();
            }
            KeyCode::Enter => actions.extend(self.activate_main()),
            _ => {}
        }
        actions
    }

    fn on_inspector_key(&mut self, key: KeyEvent) -> Vec<Action> {
        let mut actions = Vec::new();
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => self.focus = Focus::Main,
            KeyCode::Char('l') | KeyCode::Right => self.focus = Focus::Main,
            KeyCode::Char('j') | KeyCode::Down => match self.inspector_tab {
                InspectorTab::Queue => {
                    if !self.queue.is_empty() {
                        self.queue_selected = (self.queue_selected + 1).min(self.queue.len() - 1);
                    }
                }
                InspectorTab::Lyrics => {
                    self.lyrics_offset = self.lyrics_offset.saturating_add(1);
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.inspector_tab {
                InspectorTab::Queue => {
                    self.queue_selected = self.queue_selected.saturating_sub(1);
                }
                InspectorTab::Lyrics => {
                    self.lyrics_offset = self.lyrics_offset.saturating_sub(1);
                }
            },
            KeyCode::Char('g') => match self.inspector_tab {
                InspectorTab::Queue => self.queue_selected = 0,
                InspectorTab::Lyrics => self.lyrics_offset = 0,
            },
            KeyCode::Char('G') => match self.inspector_tab {
                InspectorTab::Queue => {
                    self.queue_selected = self.queue.len().saturating_sub(1);
                }
                InspectorTab::Lyrics => self.lyrics_offset = 0,
            },
            KeyCode::Enter if self.inspector_tab == InspectorTab::Queue => {
                if !self.queue.is_empty() {
                    actions.extend(self.start_queue_index(self.queue_selected));
                }
            }
            KeyCode::Tab => self.focus = self.focus.next(),
            _ => {}
        }
        actions
    }

    fn activate_main(&mut self) -> Vec<Action> {
        match &self.view {
            View::Library(Section::Playlists) => {
                let Some(playlist) = self
                    .library
                    .as_ref()
                    .and_then(|library| library.playlists.get(self.main_selected))
                    .cloned()
                else {
                    return Vec::new();
                };
                vec![self.open_collection(CollectionRequest::Playlist(playlist))]
            }
            View::Library(Section::Artists) => {
                let Some(artist) = self
                    .library
                    .as_ref()
                    .and_then(|library| library.artists.get(self.main_selected))
                    .cloned()
                else {
                    return Vec::new();
                };
                vec![self.open_collection(CollectionRequest::Artist(artist))]
            }
            View::Library(Section::Albums) => {
                let Some(album) = self
                    .library
                    .as_ref()
                    .and_then(|library| library.albums.get(self.main_selected))
                    .cloned()
                else {
                    return Vec::new();
                };
                vec![self.open_collection(CollectionRequest::Album(album))]
            }
            View::Library(Section::Home | Section::Liked) | View::Collection(_) => {
                let index = self.main_selected;
                self.start_visible_track(index)
            }
        }
    }

    fn start_visible_track(&mut self, index: usize) -> Vec<Action> {
        let tracks = self.visible_tracks().to_vec();
        if tracks.is_empty() {
            return Vec::new();
        }
        let index = index.min(tracks.len() - 1);
        self.queue = tracks;
        self.queue_cursor = index;
        self.queue_selected = index;
        self.start_current_track()
    }

    fn start_current_track(&mut self) -> Vec<Action> {
        let Some(track) = self.queue.get(self.queue_cursor).cloned() else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        self.current = Some(track.clone());
        self.duration = track.duration();
        self.position = Duration::ZERO;
        self.playing = self.mode == Mode::Demo;
        self.buffering = self.mode == Mode::Live;
        self.last_track_change = Instant::now();
        self.queue_selected = self.queue_cursor;

        self.lyrics = None;
        self.lyrics_loading = false;
        self.lyrics_error = None;
        self.lyrics_offset = 0;
        self.cover_loading = false;
        if self.show_covers {
            self.cover = Some(CoverImage::procedural(&format!(
                "{} / {}",
                track.title, track.artist
            )));
        } else {
            self.cover = None;
        }

        actions.push(Action::Playback {
            track: track.clone(),
        });

        if self.mode == Mode::Demo {
            if let Some(lyrics) = demo::lyrics_for(&track) {
                self.lyrics = Some(lyrics);
            }
        } else {
            self.lyrics_request = self.lyrics_request.wrapping_add(1);
            self.lyrics_loading = true;
            actions.push(Action::FetchLyrics {
                track: track.clone(),
                request_id: self.lyrics_request,
            });

            if self.show_covers {
                if let Some(url) = &track.cover_url {
                    self.cover_request = self.cover_request.wrapping_add(1);
                    self.cover_loading = true;
                    actions.push(Action::FetchCover {
                        url: url.clone(),
                        request_id: self.cover_request,
                    });
                }
            }
        }
        actions
    }

    fn start_queue_index(&mut self, index: usize) -> Vec<Action> {
        if index >= self.queue.len() {
            return Vec::new();
        }
        self.queue_cursor = index;
        self.start_current_track()
    }

    fn queue_step(&mut self, forward: bool, wrap: bool) -> Vec<Action> {
        if self.queue.is_empty() {
            return Vec::new();
        }
        let cursor = if forward {
            if self.queue_cursor + 1 < self.queue.len() {
                self.queue_cursor + 1
            } else if wrap {
                0
            } else {
                self.queue_cursor
            }
        } else if self.queue_cursor > 0 {
            self.queue_cursor - 1
        } else if wrap {
            self.queue.len() - 1
        } else {
            self.queue_cursor
        };
        if cursor == self.queue_cursor && self.current.is_some() {
            return Vec::new();
        }
        self.start_queue_index(cursor)
    }

    fn toggle_playback(&mut self) -> Vec<Action> {
        if self.current.is_none() {
            // Space starts the focused item when nothing is playing.
            return if self.main_len() > 0 {
                self.start_visible_track(self.main_selected)
            } else if !self.queue.is_empty() {
                self.start_queue_index(self.queue_selected)
            } else {
                Vec::new()
            };
        }
        if self.buffering {
            self.set_notice("Waiting for the track to load.", NoticeKind::Info);
            return Vec::new();
        }
        self.playing = !self.playing;
        if self.mode == Mode::Demo {
            Vec::new()
        } else if self.playing {
            vec![Action::Resume]
        } else {
            vec![Action::Pause]
        }
    }

    fn seek_relative(&mut self, delta_seconds: i64) -> Vec<Action> {
        if self.current.is_none() {
            return Vec::new();
        }
        let delta = Duration::from_secs(delta_seconds.unsigned_abs());
        self.position = if delta_seconds >= 0 {
            (self.position + delta).min(self.duration)
        } else {
            self.position.saturating_sub(delta)
        };
        if self.mode == Mode::Demo {
            Vec::new()
        } else {
            vec![Action::Seek(self.position)]
        }
    }

    fn adjust_volume(&mut self, delta: f32) -> Vec<Action> {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.muted = false;
        }
        let effective = if self.muted { 0.0 } else { self.volume };
        if self.mode == Mode::Demo {
            Vec::new()
        } else {
            vec![Action::SetVolume(effective)]
        }
    }

    fn toggle_mute(&mut self) -> Vec<Action> {
        self.muted = !self.muted;
        let effective = if self.muted { 0.0 } else { self.volume };
        self.set_notice(
            if self.muted {
                "Muted."
            } else {
                "Volume restored."
            },
            NoticeKind::Info,
        );
        if self.mode == Mode::Demo {
            Vec::new()
        } else {
            vec![Action::SetVolume(effective)]
        }
    }

    fn toggle_like(&mut self) -> Vec<Action> {
        let Some(track) = self.selected_track_for_action() else {
            return Vec::new();
        };
        let liked = !self.liked_ids.contains(&track.id);
        if liked {
            self.liked_ids.insert(track.id.clone());
            if let Some(library) = &mut self.library {
                if !library
                    .liked_tracks
                    .iter()
                    .any(|saved| saved.id == track.id)
                {
                    library.liked_tracks.push(track.clone());
                }
            }
        } else {
            self.liked_ids.remove(&track.id);
            if let Some(library) = &mut self.library {
                library.liked_tracks.retain(|saved| saved.id != track.id);
            }
        }
        self.clamp_main_selection();
        if self.mode == Mode::Demo {
            self.set_notice(
                if liked {
                    "Added to liked tracks."
                } else {
                    "Removed from liked tracks."
                },
                NoticeKind::Success,
            );
            Vec::new()
        } else {
            vec![Action::SetLike { track, liked }]
        }
    }

    fn download_selected(&mut self) -> Vec<Action> {
        let Some(track) = self.selected_track_for_action() else {
            return Vec::new();
        };
        if self.cached_ids.contains(&track.id) {
            self.set_notice("Already available offline.", NoticeKind::Info);
            return Vec::new();
        }
        if self.mode == Mode::Demo {
            self.set_notice("Demo mode: downloads are simulated.", NoticeKind::Info);
            self.cached_ids.insert(track.id);
            self.cached_count = self.cached_ids.len().max(self.cached_count);
            return Vec::new();
        }
        self.set_notice(format!("Downloading “{}”…", track.title), NoticeKind::Info);
        vec![Action::Download { track }]
    }

    fn selected_track_for_action(&self) -> Option<Track> {
        if self.now_playing {
            return self.current.clone();
        }
        if let Some(track) = self.visible_tracks().get(self.main_selected) {
            return Some(track.clone());
        }
        if self.inspector_tab == InspectorTab::Queue {
            if let Some(track) = self.queue.get(self.queue_selected) {
                return Some(track.clone());
            }
        }
        self.current.clone()
    }

    fn toggle_inspector_tab(&mut self) {
        self.inspector_tab = match self.inspector_tab {
            InspectorTab::Queue => InspectorTab::Lyrics,
            InspectorTab::Lyrics => InspectorTab::Queue,
        };
    }

    fn toggle_now_playing(&mut self) {
        if self.now_playing {
            self.now_playing = false;
        } else if self.current.is_some() {
            self.now_playing = true;
        } else {
            self.set_notice("Play a track to open Now Playing.", NoticeKind::Info);
        }
    }

    fn escape(&mut self) {
        if self.now_playing {
            self.now_playing = false;
        } else if matches!(self.view, View::Collection(_)) {
            self.back_to_library();
        } else if self.focus != Focus::Sidebar {
            self.focus = Focus::Sidebar;
        }
    }

    fn back_to_library(&mut self) {
        self.view = View::Library(self.section);
        self.main_selected = 0;
        self.focus = Focus::Main;
    }

    fn switch_section(&mut self, section: Section, keep_focus: bool) {
        self.section = section;
        self.nav_index = section.index();
        self.view = View::Library(section);
        self.main_selected = 0;
        self.now_playing = false;
        if !keep_focus {
            self.focus = Focus::Main;
        }
    }

    fn open_search(&mut self) {
        self.now_playing = false;
        self.overlay = Some(Overlay::Search(SearchState {
            query: String::new(),
            focus: SearchFocus::Input,
            results: SearchResults::default(),
            selected: 0,
            loading: false,
            error: None,
            request_id: 0,
            requested_query: String::new(),
            edited_at: Instant::now(),
        }));
    }

    fn activate_search_hit(&mut self) -> Vec<Action> {
        let Some(Overlay::Search(search)) = &self.overlay else {
            return Vec::new();
        };
        let Some(hit) = search.hit_at(search.selected) else {
            return Vec::new();
        };
        match hit.kind {
            SearchKind::Track => {
                let tracks = search.results.tracks.clone();
                let title = search.results.query.clone();
                if tracks.get(hit.index).is_none() {
                    return Vec::new();
                }
                self.overlay = None;
                self.queue = tracks;
                self.queue_cursor = hit.index;
                self.queue_selected = hit.index;
                self.set_notice(
                    format!("Playing from search for “{title}”."),
                    NoticeKind::Info,
                );
                self.start_current_track()
            }
            SearchKind::Album => {
                let Some(album) = search.results.albums.get(hit.index).cloned() else {
                    return Vec::new();
                };
                self.overlay = None;
                vec![self.open_collection(CollectionRequest::Album(album))]
            }
            SearchKind::Artist => {
                let Some(artist) = search.results.artists.get(hit.index).cloned() else {
                    return Vec::new();
                };
                self.overlay = None;
                vec![self.open_collection(CollectionRequest::Artist(artist))]
            }
            SearchKind::Playlist => {
                let Some(playlist) = search.results.playlists.get(hit.index).cloned() else {
                    return Vec::new();
                };
                self.overlay = None;
                vec![self.open_collection(CollectionRequest::Playlist(playlist))]
            }
        }
    }

    fn request_manual_retry(&mut self) {
        if let Some(plan) = &mut self.retry {
            plan.due = Some(Instant::now());
            self.set_notice("Refreshing now…", NoticeKind::Info);
        } else if let Some(error) = self.content_error().map(str::to_owned) {
            self.set_notice(
                format!("Nothing to retry automatically: {error}"),
                NoticeKind::Info,
            );
        } else {
            self.set_notice("Everything is up to date.", NoticeKind::Info);
        }
    }
}

impl SearchState {
    /// Number of selectable hits.
    #[must_use]
    pub fn result_count(&self) -> usize {
        self.results.total()
    }

    /// Resolve a flat selection to a category and local index.
    #[must_use]
    pub fn hit_at(&self, selected: usize) -> Option<SearchHit> {
        if selected < self.results.tracks.len() {
            return Some(SearchHit {
                kind: SearchKind::Track,
                index: selected,
            });
        }
        let mut offset = self.results.tracks.len();
        if selected < offset + self.results.albums.len() {
            return Some(SearchHit {
                kind: SearchKind::Album,
                index: selected - offset,
            });
        }
        offset += self.results.albums.len();
        if selected < offset + self.results.artists.len() {
            return Some(SearchHit {
                kind: SearchKind::Artist,
                index: selected - offset,
            });
        }
        offset += self.results.artists.len();
        if selected < offset + self.results.playlists.len() {
            return Some(SearchHit {
                kind: SearchKind::Playlist,
                index: selected - offset,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn demo_state_has_a_queue_and_synced_lyrics() {
        let app = App::demo(&config());
        assert_eq!(app.queue.len(), 5);
        assert!(app.signed_in);
        assert!(app.lyrics.as_ref().is_some_and(|lyrics| lyrics.synced));
        assert!(app.position > Duration::from_secs(100));
    }

    #[test]
    fn navigation_preserves_a_valid_selection() {
        let mut app = App::demo(&config());
        app.focus = Focus::Main;
        for _ in 0..40 {
            let _ = app.on_key(KeyEvent::from(KeyCode::Char('j')));
        }
        assert!(app.main_selected < app.main_len());
    }

    #[test]
    fn search_debounce_is_driven_by_ticks() {
        let mut app = App::demo(&config());
        let _ = app.on_key(KeyEvent::from(KeyCode::Char('/')));
        for ch in "ночной".chars() {
            let _ = app.on_key(KeyEvent::from(KeyCode::Char(ch)));
        }
        std::thread::sleep(Duration::from_millis(340));
        let actions = app.tick(Duration::from_millis(100));
        assert!(actions
            .iter()
            .any(|action| matches!(action, Action::Search { query, .. } if query == "ночной")));
    }

    #[test]
    fn playback_moves_through_the_queue_and_stops_at_the_end() {
        let mut app = App::demo(&config());
        app.queue.truncate(2);
        app.playing = true;
        app.position = app.duration - Duration::from_millis(10);
        let actions = app.tick(Duration::from_millis(50));
        assert!(matches!(actions.last(), Some(Action::Playback { .. })));
        assert_eq!(app.queue_cursor, 1);
        app.playing = true;
        app.position = app.duration - Duration::from_millis(10);
        let _ = app.tick(Duration::from_millis(50));
        assert!(!app.playing);
    }
}
