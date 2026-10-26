//! the reconnecting realtime client.
//!
//! ynison is the only server-push channel yandex music offers — there are no
//! webhooks — so a "now playing" watcher lives or dies on this connection staying
//! up. [`Realtime`] owns that: it reconnects with backoff, honours (but caps)
//! server "go away" delays, and tears down a silently-dead socket.
//!
//! events arrive on a [`broadcast`] channel, so any number of subscribers can
//! listen, and [`Realtime::now_playing`] answers synchronously from the last
//! frame — handy for serving an http request without waiting for the next update.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::{broadcast, mpsc, watch};

use super::messages::{DeviceInfo, generate_device_id};
use super::session::{self, SessionConfig};
use super::state::{PlayerState, now_ms};

/// what [`Realtime`] emits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// a session was established.
    Connected,
    /// a player-state frame arrived.
    State(PlayerState),
    /// the current track changed. carries the new playable id.
    TrackChanged(String),
    /// playback was paused (`true`) or resumed (`false`).
    PauseChanged(bool),
    /// a recoverable failure; the client will reconnect if still running.
    Error(String),
    /// a reconnect is scheduled after this delay.
    Reconnecting(Duration),
    /// the socket went silent past the stale timeout and is being torn down.
    /// carries how long it had been idle.
    Stale(Duration),
    /// the client stopped and will not reconnect.
    Closed,
}

/// a synchronous snapshot of what is playing right now.
#[derive(Debug, Clone, PartialEq)]
pub struct NowPlaying {
    /// the current playable id, or `None` when nothing is playing.
    pub playable_id: Option<String>,
    /// whether playback is paused.
    pub paused: bool,
    /// track length in milliseconds.
    pub duration_ms: i64,
    /// playback position, extrapolated to the moment of the call.
    pub progress_ms: i64,
    /// the current entity id: album, playlist, ...
    pub entity_id: Option<String>,
    /// the current entity type.
    pub entity_type: Option<String>,
    /// milliseconds since the frame this snapshot came from.
    pub age_ms: i64,
}

/// configuration for [`Realtime`].
#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    /// oauth token.
    pub token: String,
    /// device id. defaults to a random one, generated once and reused.
    ///
    /// keeping it stable across reconnects matters: ynison deduplicates its
    /// broadcast by device, so a fresh id each reconnect makes the server drop
    /// this observer from the fan-out and the state goes silent.
    pub device_id: Option<String>,
    /// device identity to present. android exposes the most state.
    pub device: DeviceInfo,
    /// whether to reconnect automatically.
    pub reconnect: bool,
    /// initial reconnect backoff.
    pub reconnect_base: Duration,
    /// reconnect backoff ceiling.
    pub reconnect_max: Duration,
    /// cap on a server-requested "go away" delay.
    ///
    /// ynison answers "temporarily unavailable" with a go-away of an hour or more.
    /// honouring that verbatim leaves a watcher dead long after the service
    /// returns; capping it recovers within a minute while still backing off.
    pub go_away_max: Duration,
    /// force a reconnect when the socket shows no activity for this long.
    ///
    /// `None` disables the watchdog. this watches transport liveness, kept fresh
    /// by the ~20s keepalive ping — not state-frame arrival, which is legitimately
    /// silent while a player is paused. tearing a connection down on frame silence
    /// causes a reconnect storm the server answers with rate limiting.
    pub stale_timeout: Option<Duration>,
}

impl RealtimeConfig {
    /// configure a realtime client for a token, with everything else defaulted.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            device_id: None,
            device: DeviceInfo::ANDROID,
            reconnect: true,
            reconnect_base: Duration::from_secs(1),
            reconnect_max: Duration::from_secs(30),
            go_away_max: Duration::from_secs(60),
            stale_timeout: Some(Duration::from_secs(120)),
        }
    }

    /// pin the device id instead of generating one.
    #[must_use]
    pub fn device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// present a different device identity.
    #[must_use]
    pub fn device(mut self, device: DeviceInfo) -> Self {
        self.device = device;
        self
    }

    /// run a single session and stop, rather than reconnecting.
    #[must_use]
    pub fn once(mut self) -> Self {
        self.reconnect = false;
        self
    }

    /// set the stale-socket watchdog, or `None` to disable it.
    #[must_use]
    pub fn stale_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.stale_timeout = timeout;
        self
    }
}

#[derive(Debug, Default)]
struct Shared {
    last_state: Option<PlayerState>,
    last_state_at: i64,
}

/// a reconnecting realtime player-state client.
///
/// # examples
///
/// ```no_run
/// use yamuse::ynison::{Realtime, RealtimeConfig, Event};
///
/// # async fn run() -> Result<(), yamuse::Error> {
/// let realtime = Realtime::start(RealtimeConfig::new("your-token"));
/// let mut events = realtime.subscribe();
///
/// while let Ok(event) = events.recv().await {
///     match event {
///         Event::TrackChanged(id) => println!("now playing: {id}"),
///         Event::PauseChanged(paused) => println!("paused: {paused}"),
///         Event::Closed => break,
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Realtime {
    events: broadcast::Sender<Event>,
    shared: Arc<RwLock<Shared>>,
    device_id: String,
    /// `true` once [`Realtime::stop`] has been called.
    ///
    /// a watch channel rather than an `AtomicBool`, so a waiter can be woken the
    /// moment it flips instead of polling for it. the value is retained, so a
    /// `stop()` that lands before anyone waits is still seen.
    stop: watch::Sender<bool>,
}

impl Realtime {
    /// start the client, spawning its loop on the current runtime.
    ///
    /// the returned handle can be dropped without stopping the loop; call
    /// [`stop`](Self::stop) for that.
    #[must_use]
    pub fn start(config: RealtimeConfig) -> Arc<Self> {
        // 64 is generous for a control channel: a slow subscriber lags rather than
        // blocking the socket, and `Lagged` tells it frames were skipped.
        let (events, _) = broadcast::channel(64);
        let device_id = config.device_id.clone().unwrap_or_else(generate_device_id);

        let realtime = Arc::new(Self {
            events: events.clone(),
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: device_id.clone(),
            stop: watch::channel(false).0,
        });

        let driver = Arc::clone(&realtime);
        tokio::spawn(async move { driver.run(config, device_id).await });
        realtime
    }

    /// subscribe to the event stream.
    ///
    /// each subscriber gets every event from the moment it subscribes.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// the stable device id this client presents.
    #[must_use]
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// the last state frame received, if any.
    #[must_use]
    pub fn state(&self) -> Option<PlayerState> {
        self.shared.read().ok()?.last_state.clone()
    }

    /// milliseconds since the last state frame, or `None` if none arrived yet.
    #[must_use]
    pub fn last_state_age_ms(&self) -> Option<i64> {
        let shared = self.shared.read().ok()?;
        (shared.last_state_at > 0).then(|| now_ms() - shared.last_state_at)
    }

    /// a snapshot of what is playing right now, with the position extrapolated.
    #[must_use]
    pub fn now_playing(&self) -> Option<NowPlaying> {
        let shared = self.shared.read().ok()?;
        let state = shared.last_state.as_ref()?;
        Some(NowPlaying {
            playable_id: state
                .current_playable_id()
                .map(std::borrow::ToOwned::to_owned),
            paused: state.paused,
            duration_ms: state.duration_ms,
            progress_ms: state.live_progress_ms(),
            entity_id: state.entity_id.clone(),
            entity_type: state.entity_type.clone(),
            age_ms: now_ms() - shared.last_state_at,
        })
    }

    /// whether the loop is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        !*self.stop.borrow()
    }

    /// stop the loop. the next event emitted is [`Event::Closed`].
    pub fn stop(&self) {
        // `send_replace`, not `send`: the latter reports failure when no receiver
        // is currently subscribed, and would leave the flag unset for a client
        // nobody happens to be waiting on.
        self.stop.send_replace(true);
    }

    /// resolves once [`stop`](Self::stop) has been called — immediately if it
    /// already has.
    ///
    /// this is what lets a reconnect wait be interrupted instead of polled. the
    /// watch channel retains its value, so there is no window in which a `stop()`
    /// can be missed between the check and the await.
    async fn stopped(&self) {
        let mut rx = self.stop.subscribe();
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                return; // the sender is gone, so nothing can run any more either
            }
        }
    }

    /// emit an event, ignoring the "nobody is subscribed" case.
    fn emit(&self, event: Event) {
        #[cfg(feature = "tracing")]
        log_event(&self.device_id, &event);
        let _ = self.events.send(event);
    }

    /// the connect-then-reconnect loop.
    async fn run(self: Arc<Self>, config: RealtimeConfig, device_id: String) {
        let session_config = SessionConfig {
            token: config.token.clone(),
            device_id,
            device: config.device.clone(),
        };
        let mut attempt: u32 = 0;
        let mut last_playable: Option<String> = None;
        let mut last_paused: Option<bool> = None;

        while self.is_running() {
            let (states_tx, mut states_rx) = mpsc::channel::<PlayerState>(32);
            let handle = session::new_handle();

            // the watchdog cannot live inside the session: it must be able to
            // outlive a socket that has stopped producing anything at all.
            let watchdog = config.stale_timeout.map(|timeout| {
                let handle = handle.clone();
                let realtime = Arc::clone(&self);
                tokio::spawn(async move {
                    let tick = timeout.max(Duration::from_secs(1)) / 4;
                    loop {
                        tokio::time::sleep(tick).await;
                        // a negative idle would mean the clock jumped backwards; treat
                        // that as "just saw activity" rather than as a dead socket.
                        let idle = u64::try_from(handle.idle_ms()).unwrap_or(0);
                        if Duration::from_millis(idle) > timeout {
                            realtime.emit(Event::Stale(Duration::from_millis(idle)));
                            return; // dropping the session future closes the socket
                        }
                        if !realtime.is_running() {
                            return;
                        }
                    }
                })
            });

            let session = session::run(&session_config, states_tx, handle);
            let mut opened = false;
            let outcome = {
                tokio::pin!(session);
                loop {
                    tokio::select! {
                        result = &mut session => break result,
                        Some(state) = states_rx.recv() => {
                            if !opened {
                                opened = true;
                                attempt = 0;
                                self.emit(Event::Connected);
                            }
                            self.handle_state(state, &mut last_playable, &mut last_paused);
                        },
                        // a stop() is noticed the instant it lands, even while
                        // the socket is silent — no polling cadence to lag behind.
                        () = self.stopped() => break Ok(()),
                    }
                }
            };

            if let Some(watchdog) = watchdog {
                watchdog.abort();
            }

            let delay = match outcome {
                Ok(()) => Self::backoff(&config, &mut attempt),
                Err(err) => {
                    let capped = err
                        .retry_after()
                        .map(|requested| requested.min(config.go_away_max));
                    self.emit(Event::Error(err.to_string()));
                    capped.unwrap_or_else(|| Self::backoff(&config, &mut attempt))
                },
            };

            if !self.is_running() || !config.reconnect {
                break;
            }

            self.emit(Event::Reconnecting(delay));
            // wake early if stop() lands mid-wait, rather than sleeping it out
            tokio::select! {
                () = tokio::time::sleep(delay) => {},
                () = self.stopped() => {},
            }
        }

        self.stop();
        self.emit(Event::Closed);
    }

    /// record a frame and emit the derived change events.
    fn handle_state(
        &self,
        state: PlayerState,
        last_playable: &mut Option<String>,
        last_paused: &mut Option<bool>,
    ) {
        let playable = state
            .current_playable_id()
            .map(std::borrow::ToOwned::to_owned);
        let paused = state.paused;

        if let Ok(mut shared) = self.shared.write() {
            shared.last_state = Some(state.clone());
            shared.last_state_at = now_ms();
        }
        self.emit(Event::State(state));

        if *last_paused != Some(paused) {
            *last_paused = Some(paused);
            self.emit(Event::PauseChanged(paused));
        }
        if let Some(id) = playable {
            if last_playable.as_deref() != Some(id.as_str()) {
                *last_playable = Some(id.clone());
                self.emit(Event::TrackChanged(id));
            }
        }
    }

    fn backoff(config: &RealtimeConfig, attempt: &mut u32) -> Duration {
        let exp = config
            .reconnect_base
            .saturating_mul(1u32 << (*attempt).min(16));
        *attempt = attempt.saturating_add(1);
        exp.min(config.reconnect_max)
    }
}

/// mirror the event stream into `tracing`.
///
/// every event already flows through `emit`, so one sink here covers the whole
/// lifecycle: connects, errors, the stale-socket teardown and each reconnect.
#[cfg(feature = "tracing")]
fn log_event(device_id: &str, event: &Event) {
    match event {
        Event::Error(message) => {
            crate::log::warning!(device_id, "ynison session failed: {message}");
        },
        Event::Stale(idle) => {
            crate::log::warning!(
                device_id,
                idle_ms = idle.as_millis(),
                "ynison socket went silent; tearing it down",
            );
        },
        Event::Reconnecting(delay) => {
            crate::log::debug!(
                device_id,
                delay_ms = delay.as_millis(),
                "ynison reconnecting"
            );
        },
        Event::Connected => crate::log::debug!(device_id, "ynison connected"),
        Event::Closed => crate::log::debug!(device_id, "ynison closed"),
        // state frames arrive continuously; logging each would drown the rest
        Event::State(_) | Event::TrackChanged(_) | Event::PauseChanged(_) => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_reconnects_and_watches_for_staleness() {
        let config = RealtimeConfig::new("t");
        assert!(config.reconnect);
        assert_eq!(config.device, DeviceInfo::ANDROID);
        assert_eq!(config.stale_timeout, Some(Duration::from_secs(120)));
        assert_eq!(config.go_away_max, Duration::from_secs(60));
        assert!(config.device_id.is_none());
    }

    #[test]
    fn config_builders_chain() {
        let config = RealtimeConfig::new("t")
            .device_id("fixed")
            .device(DeviceInfo::WEB)
            .stale_timeout(None)
            .once();
        assert_eq!(config.device_id.as_deref(), Some("fixed"));
        assert_eq!(config.device, DeviceInfo::WEB);
        assert!(config.stale_timeout.is_none());
        assert!(!config.reconnect);
    }

    #[test]
    fn backoff_grows_then_settles_at_the_ceiling() {
        let config = RealtimeConfig::new("t");
        let mut attempt = 0;
        let delays: Vec<Duration> = (0..8)
            .map(|_| Realtime::backoff(&config, &mut attempt))
            .collect();

        assert_eq!(delays[0], Duration::from_secs(1));
        assert_eq!(delays[1], Duration::from_secs(2));
        assert_eq!(delays[2], Duration::from_secs(4));
        assert!(delays.iter().all(|d| *d <= config.reconnect_max));
        assert_eq!(*delays.last().unwrap(), config.reconnect_max);
    }

    #[tokio::test]
    async fn state_changes_emit_track_and_pause_events_once_each() {
        let (events, mut rx) = broadcast::channel(32);
        let realtime = Realtime {
            events,
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: "d".into(),
            stop: watch::channel(false).0,
        };

        let frame = |id: &str, paused: bool| PlayerState {
            playable_list: vec![super::super::state::Playable {
                playable_id: Some(id.to_owned()),
            }],
            current_playable_index: 0,
            paused,
            playback_speed: 1.0,
            ..Default::default()
        };

        let mut last_playable = None;
        let mut last_paused = None;

        realtime.handle_state(frame("111", false), &mut last_playable, &mut last_paused);
        // an identical frame must not re-announce the same track or pause state
        realtime.handle_state(frame("111", false), &mut last_playable, &mut last_paused);
        realtime.handle_state(frame("222", true), &mut last_playable, &mut last_paused);

        let mut track_changes = Vec::new();
        let mut pause_changes = Vec::new();
        let mut states = 0;
        while let Ok(event) = rx.try_recv() {
            match event {
                Event::TrackChanged(id) => track_changes.push(id),
                Event::PauseChanged(paused) => pause_changes.push(paused),
                Event::State(_) => states += 1,
                _ => {},
            }
        }

        assert_eq!(track_changes, vec!["111".to_string(), "222".to_string()]);
        assert_eq!(pause_changes, vec![false, true]);
        assert_eq!(states, 3, "every frame is still forwarded verbatim");
    }

    #[tokio::test]
    async fn now_playing_reflects_the_last_frame() {
        let realtime = Realtime {
            events: broadcast::channel(8).0,
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: "d".into(),
            stop: watch::channel(false).0,
        };
        assert!(realtime.now_playing().is_none());
        assert!(realtime.last_state_age_ms().is_none());

        realtime.handle_state(
            PlayerState {
                playable_list: vec![super::super::state::Playable {
                    playable_id: Some("42".into()),
                }],
                current_playable_index: 0,
                paused: true,
                duration_ms: 200_000,
                progress_ms: 1_000,
                playback_speed: 1.0,
                entity_type: Some("ALBUM".into()),
                ..Default::default()
            },
            &mut None,
            &mut None,
        );

        let now = realtime.now_playing().unwrap();
        assert_eq!(now.playable_id.as_deref(), Some("42"));
        assert!(now.paused);
        assert_eq!(now.duration_ms, 200_000);
        assert_eq!(now.progress_ms, 1_000, "a paused position does not drift");
        assert_eq!(now.entity_type.as_deref(), Some("ALBUM"));
        assert!(now.age_ms < 1000);
        assert!(realtime.last_state_age_ms().is_some());
    }

    #[tokio::test]
    async fn a_waiter_is_woken_the_moment_stop_lands() {
        let realtime = Arc::new(Realtime {
            events: broadcast::channel(4).0,
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: "d".into(),
            stop: watch::channel(false).0,
        });

        // the reconnect wait is a select between the backoff and this; it used to
        // be a 100ms poll loop, so a stop() could sit unnoticed for that long
        let waiter = Arc::clone(&realtime);
        let handle = tokio::spawn(async move { waiter.stopped().await });
        tokio::task::yield_now().await;
        realtime.stop();

        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .expect("stop() must wake the waiter, not be polled for")
            .unwrap();
    }

    #[tokio::test]
    async fn a_stop_that_lands_before_anyone_waits_is_still_seen() {
        // the race the old atomic had: stop() between the check and the await.
        // a watch channel retains the value, so the wait resolves immediately.
        let realtime = Realtime {
            events: broadcast::channel(4).0,
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: "d".into(),
            stop: watch::channel(false).0,
        };
        realtime.stop();

        tokio::time::timeout(Duration::from_secs(1), realtime.stopped())
            .await
            .expect("a wait started after stop() must not block");
    }

    #[tokio::test]
    async fn stopping_flips_the_running_flag() {
        let realtime = Realtime {
            events: broadcast::channel(4).0,
            shared: Arc::new(RwLock::new(Shared::default())),
            device_id: "d".into(),
            stop: watch::channel(false).0,
        };
        assert!(realtime.is_running());
        realtime.stop();
        assert!(!realtime.is_running());
    }
}
