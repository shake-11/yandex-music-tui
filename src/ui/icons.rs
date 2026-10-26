//! Glyphs. Most are Font Awesome codepoints included in Nerd Fonts; core
//! playback and list markers use widely available Unicode so the interface stays
//! legible even with a partial font.

pub const HOME: &str = "\u{f015}";
pub const HEART: &str = "\u{f004}";
pub const HEART_O: &str = "\u{f08a}";
pub const PLAYLIST: &str = "\u{f03a}";
pub const ARTIST: &str = "\u{f007}";
pub const ALBUM: &str = "\u{f001}";
pub const SEARCH: &str = "\u{f002}";
pub const PLAY: &str = "\u{f04b}";
pub const PAUSE: &str = "\u{f04c}";
pub const PREVIOUS: &str = "\u{f048}";
pub const NEXT: &str = "\u{f051}";
pub const VOLUME: &str = "\u{f028}";
pub const MUTED: &str = "\u{f026}";
pub const DOWNLOAD: &str = "\u{f019}";
pub const CHECK: &str = "\u{f00c}";
pub const HEADPHONES: &str = "\u{f025}";
pub const SYNC: &str = "\u{f021}";
pub const DOT: &str = "\u{f111}";
pub const WARNING: &str = "\u{f071}";
pub const CLOSE: &str = "\u{f00d}";
pub const QUEUE: &str = "\u{f03a}";
pub const LYRICS: &str = "\u{f130}";
pub const CLOCK: &str = "\u{f017}";

/// Small equaliser used to animate the active track.
pub const EQUALISER: [&str; 4] = ["▁▃▅", "▃▅▂", "▅▂▆", "▂▆▃"];

/// Braille spinner frames.
pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
