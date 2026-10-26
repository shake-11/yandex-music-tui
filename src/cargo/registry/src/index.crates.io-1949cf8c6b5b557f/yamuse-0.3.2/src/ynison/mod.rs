//! ynison — realtime player state over websocket.
//!
//! this is the only server-push channel yandex music offers; there are no
//! webhooks. it reports what the account is playing right now, on any device.
//!
//! most callers want [`Realtime`]: it owns the handshake, the keepalive, and
//! reconnection, and hands out events on a broadcast channel.
//!
//! ```no_run
//! use yamuse::ynison::{Event, Realtime, RealtimeConfig};
//!
//! # async fn run() -> Result<(), yamuse::Error> {
//! let realtime = Realtime::start(RealtimeConfig::new("your-token"));
//! let mut events = realtime.subscribe();
//!
//! while let Ok(event) = events.recv().await {
//!     if let Event::TrackChanged(playable_id) = event {
//!         println!("now playing: {playable_id}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [`session`] exposes the single-session primitive underneath, for callers that
//! want to drive reconnection themselves.

pub mod messages;
pub mod realtime;
pub mod session;
pub mod state;

pub use messages::{DeviceInfo, generate_device_id, generate_request_id, update_full_state};
pub use realtime::{Event, NowPlaying, Realtime, RealtimeConfig};
pub use session::{SessionConfig, SessionHandle};
pub use state::{Playable, PlayerState, Redirect, parse_redirect_frame, parse_state_frame};

use crate::client::Client;

impl Client {
    /// start a realtime player-state client using this client's token.
    ///
    /// # errors
    ///
    /// returns [`crate::Error::Unauthorized`] when the client has no token.
    pub fn realtime(&self) -> crate::Result<std::sync::Arc<Realtime>> {
        let token = self
            .token()
            .ok_or_else(|| crate::Error::Unauthorized("the realtime api needs a token".into()))?;
        Ok(Realtime::start(RealtimeConfig::new(token)))
    }

    /// start a realtime client with explicit configuration.
    #[must_use]
    pub fn realtime_with(&self, config: RealtimeConfig) -> std::sync::Arc<Realtime> {
        Realtime::start(config)
    }
}
