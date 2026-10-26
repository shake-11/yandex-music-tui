//! LRC parsing and current-line selection.
//!
//! Yandex serves synchronised lyrics as LRC behind a second file URL. LRC is
//! deceptively simple, so this parser accepts the shapes seen in the wild:
//! multiple timestamps per line, metadata tags, colons used for milliseconds,
//! an offset tag, and a plain-text fallback when timestamps are absent.

use std::time::Duration;

/// One line of lyrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    /// Synchronisation point, if the source was time-coded.
    pub at: Option<Duration>,
    pub text: String,
}

/// Parsed lyrics for the currently playing track.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// True when the source carried LRC timestamps.
    pub synced: bool,
    pub writers: Vec<String>,
}

impl Lyrics {
    /// Parse an LRC document, falling back to plain text when it has no usable
    /// timestamps.
    #[must_use]
    pub fn parse_lrc(raw: &str) -> Self {
        let offset = parse_offset(raw);
        let mut lines: Vec<LyricLine> = Vec::new();

        for source_line in raw.lines() {
            let source_line = source_line.trim_end_matches('\r').trim();
            if source_line.is_empty() {
                continue;
            }

            let mut rest = source_line;
            let mut stamps: Vec<Duration> = Vec::new();
            while let Some(line) = rest.strip_prefix('[') {
                let Some(end) = line.find(']') else {
                    break;
                };
                let tag = &line[..end];
                match parse_timestamp(tag) {
                    Some(mut timestamp) => {
                        if offset.is_negative() {
                            timestamp = timestamp
                                .saturating_sub(Duration::from_millis(offset.unsigned_abs()));
                        } else {
                            timestamp += Duration::from_millis(offset.unsigned_abs());
                        }
                        stamps.push(timestamp);
                        rest = &line[end + 1..];
                    }
                    None => {
                        // Metadata such as [ar:...] or [offset:...]. Consume it,
                        // but do not emit it as a lyric.
                        rest = &line[end + 1..];
                    }
                }
            }

            let text = rest.trim().to_owned();
            if text.is_empty() || stamps.is_empty() {
                continue;
            }
            // A BOM or zero-width space can arrive at the start of the first line.
            let text = text.trim_start_matches(['\u{feff}', '\u{200b}']).to_owned();
            for at in stamps {
                lines.push(LyricLine {
                    at: Some(at),
                    text: text.clone(),
                });
            }
        }

        if lines.is_empty() {
            return Self::parse_plain(raw);
        }
        lines.sort_by_key(|line| line.at);
        Self {
            lines,
            synced: true,
            writers: Vec::new(),
        }
    }

    /// Turn free text into unsynchronised lyrics.
    #[must_use]
    pub fn parse_plain(raw: &str) -> Self {
        let lines = raw
            .lines()
            .map(|line| line.trim_end_matches('\r').trim())
            .filter(|line| !line.is_empty())
            .map(|text| LyricLine {
                at: None,
                text: text.to_owned(),
            })
            .collect::<Vec<_>>();
        Self {
            lines,
            synced: false,
            writers: Vec::new(),
        }
    }

    /// Whether this document contains anything to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Index of the line that should be highlighted for `position`.
    ///
    /// For plain lyrics this interpolates through the song so the view still
    /// tracks progress, even though no timestamp is authoritative.
    #[must_use]
    pub fn active_index(&self, position: Duration, duration: Duration) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        if self.synced {
            let timestamps = self
                .lines
                .iter()
                .map(|line| line.at.unwrap_or_default())
                .collect::<Vec<_>>();
            let index = timestamps.partition_point(|at| *at <= position);
            return Some(index.saturating_sub(1));
        }

        let duration = duration.as_secs_f64();
        if duration <= f64::EPSILON {
            return Some(0);
        }
        let ratio = (position.as_secs_f64() / duration).clamp(0.0, 0.999_999);
        let index = (ratio * self.lines.len() as f64) as usize;
        Some(index.min(self.lines.len().saturating_sub(1)))
    }
}

fn parse_offset(raw: &str) -> i64 {
    for line in raw.lines() {
        let line = line.trim();
        let Some(inner) = line.strip_prefix("[offset:") else {
            continue;
        };
        let Some(value) = inner.strip_suffix(']') else {
            continue;
        };
        if let Ok(offset) = value.trim().parse::<i64>() {
            return offset.clamp(-60_000, 60_000);
        }
    }
    0
}

/// Parse `mm:ss`, `mm:ss.mmm`, `mm:ss:xx`, or `hh:mm:ss.xx`.
fn parse_timestamp(tag: &str) -> Option<Duration> {
    if !tag.contains(':') || tag.chars().any(|ch| ch.is_alphabetic()) {
        return None;
    }

    let normalized = tag.replace(',', ".");
    let parts: Vec<&str> = normalized.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }

    let (minutes, seconds, fraction) = match parts.as_slice() {
        [minutes, seconds] => {
            let (seconds, fraction) = seconds
                .split_once('.')
                .map_or((*seconds, None), |(seconds, fraction)| {
                    (seconds, Some(fraction))
                });
            (*minutes, seconds, fraction)
        }
        [minutes, seconds, fraction] => (*minutes, *seconds, Some(*fraction)),
        _ => return None,
    };

    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<u64>().ok()?;
    if seconds >= 60 {
        return None;
    }

    let millis = match fraction {
        None => 0,
        Some(value) => {
            let digits = value.chars().take(3).collect::<String>();
            let value = digits.parse::<u64>().ok()?;
            match digits.len() {
                0 => 0,
                1 => value * 100,
                2 => value * 10,
                _ => value,
            }
        }
    };

    Some(Duration::from_millis(
        minutes * 60_000 + seconds * 1_000 + millis,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_metadata_is_ignored_and_multiple_stamps_are_expanded() {
        let raw = "\
[ar:Someone]
[ti:Song]
[00:01.00][00:31.50]First line
[00:05.250]Second line
";
        let lyrics = Lyrics::parse_lrc(raw);
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].text, "First line");
        assert_eq!(lyrics.lines[0].at, Some(Duration::from_secs(1)));
        assert_eq!(lyrics.lines[2].at, Some(Duration::from_millis(31_500)));
    }

    #[test]
    fn active_line_uses_the_most_recent_timestamp() {
        let lyrics = Lyrics::parse_lrc("[00:01.00]one\n[00:05.00]two\n[00:09.00]three\n");
        assert_eq!(
            lyrics.active_index(Duration::ZERO, Duration::from_secs(12)),
            Some(0)
        );
        assert_eq!(
            lyrics.active_index(Duration::from_millis(5_200), Duration::from_secs(12)),
            Some(1)
        );
        assert_eq!(
            lyrics.active_index(Duration::from_secs(30), Duration::from_secs(12)),
            Some(2)
        );
    }

    #[test]
    fn offset_is_applied_with_both_signs() {
        let positive = Lyrics::parse_lrc("[offset:+500]\n[00:01.00]one\n");
        assert_eq!(positive.lines[0].at, Some(Duration::from_millis(1_500)));

        let negative = Lyrics::parse_lrc("[offset:-500]\n[00:01.00]one\n");
        assert_eq!(negative.lines[0].at, Some(Duration::from_millis(500)));
    }

    #[test]
    fn plain_text_falls_back_and_progress_tracks_the_song() {
        let lyrics = Lyrics::parse_lrc("first\n\nsecond\nthird\n");
        assert!(!lyrics.synced);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(
            lyrics.active_index(Duration::from_secs(5), Duration::from_secs(10)),
            Some(1)
        );
    }

    #[test]
    fn colon_fraction_is_accepted() {
        let lyrics = Lyrics::parse_lrc("[00:12:500]line\n");
        assert_eq!(lyrics.lines[0].at, Some(Duration::from_millis(12_500)));
    }
}
