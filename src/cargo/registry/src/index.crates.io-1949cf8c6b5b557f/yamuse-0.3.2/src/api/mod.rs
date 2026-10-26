//! api methods, grouped by domain.
//!
//! each module adds an `impl Client` block, so every endpoint is reachable
//! directly on the client (`client.tracks(...)`, `client.search(...)`) — one flat
//! surface, one file per domain.
//!
//! # paged endpoints
//!
//! endpoints that answer "everything about this one thing" — an artist's tracks,
//! an artist's albums, the account's liked clips — also come with an `_all`
//! variant that walks every page and concatenates the result.
//!
//! the browse-style listings (metatags, labels, concerts) deliberately do not.
//! those back infinite-scroll UIs over catalogue-sized result sets, where reading
//! every page is a mistake rather than a convenience; drive them a page at a time
//! and stop when [`Pager::has_next`](crate::models::Pager::has_next) says to.

/// how many pages an `_all` walk will read before giving up.
///
/// an endpoint that misreports its pager — always answering page 0, say — would
/// otherwise loop until the process is killed. at the page sizes these endpoints
/// use this ceiling is far past any real artist or library.
pub(crate) const MAX_PAGES: u32 = 100;

mod account;
mod albums;
mod artists;
mod clips;
mod concerts;
mod credits;
mod device_auth;
mod disclaimers;
mod labels;
mod landing;
mod likes;
mod metatags;
mod music_history;
mod pins;
mod playlists;
mod presaves;
mod queue;
mod radio;
mod search;
mod tracks;

pub use artists::ArtistAlbumsQuery;
pub use device_auth::DeviceAuthOptions;
pub use metatags::{MetatagListQuery, MetatagQuery};
pub use music_history::HistoryQuery;
pub use radio::RotorFeedbackReport;
pub use search::SearchQuery;
pub use tracks::PlayAudioReport;
