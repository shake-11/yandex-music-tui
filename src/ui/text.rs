//! Text measurement and small formatting helpers built on display width, not
//! bytes, so Cyrillic titles and wide glyphs lay out correctly.

use std::time::Duration;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Terminal display width of a string.
#[must_use]
pub fn width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Truncate to `max` display columns, adding an ellipsis when needed.
#[must_use]
pub fn truncate(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if width(text) <= max {
        return text.to_owned();
    }
    if max == 1 {
        return "…".to_owned();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > max - 1 {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push('…');
    out
}

/// Truncate, then pad with spaces to exactly `max` columns.
#[must_use]
pub fn fit(text: &str, max: usize) -> String {
    let mut out = truncate(text, max);
    let current = width(&out);
    if current < max {
        out.extend(std::iter::repeat_n(' ', max - current));
    }
    out
}

/// Keep the right-hand side of a string (useful for paths and long albums).
#[must_use]
pub fn tail(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if width(text) <= max {
        return text.to_owned();
    }
    if max == 1 {
        return "…".to_owned();
    }
    let mut chars = Vec::new();
    let mut used = 0;
    for ch in text.chars().rev() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > max - 1 {
            break;
        }
        chars.push(ch);
        used += ch_width;
    }
    chars.reverse();
    format!("…{}", chars.into_iter().collect::<String>())
}

/// `m:ss`, or `h:mm:ss` for very long audio.
#[must_use]
pub fn duration(position: Duration) -> String {
    let seconds = position.as_secs();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    if minutes >= 60 {
        format!("{}:{:02}:{:02}", minutes / 60, minutes % 60, seconds)
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Approximate total listening time for collection headings.
#[must_use]
pub fn total_time(milliseconds: u64) -> String {
    let minutes = milliseconds / 60_000;
    if minutes >= 60 {
        format!("{} h {} min", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    }
}

/// Compact count used on tiles and group headers.
#[must_use]
pub fn count(value: usize) -> String {
    if value >= 10_000 {
        format!("{:.0}k", value as f32 / 1_000.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f32 / 1_000.0)
    } else {
        value.to_string()
    }
}

/// Wrap at word boundaries, preserving explicit newlines. Never returns a line
/// wider than `width`.
#[must_use]
pub fn wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let word_width = width(word);
            if current.is_empty() {
                if word_width <= max_width {
                    current.push_str(word);
                } else {
                    current.push_str(&truncate(word, max_width));
                }
            } else if width(&current) + 1 + word_width <= max_width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                if word_width <= max_width {
                    current.push_str(word);
                } else {
                    current.push_str(&truncate(word, max_width));
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

/// Parse a rough ratio without panicking on zero-duration tracks.
#[must_use]
pub fn fraction(position: Duration, total: Duration) -> f64 {
    let total = total.as_secs_f64();
    if total <= f64::EPSILON {
        0.0
    } else {
        (position.as_secs_f64() / total).clamp(0.0, 1.0)
    }
}

/// Index of the first visible row so `selected` stays comfortably on screen.
#[must_use]
pub fn window_start(selected: usize, length: usize, visible: usize) -> usize {
    if visible == 0 || length <= visible {
        return 0;
    }
    let half = visible / 2;
    selected.saturating_sub(half).min(length - visible)
}

/// Pad to a width using a right-aligned suffix on the same line.
#[must_use]
pub fn left_and_right(left: &str, right: &str, max_width: usize) -> String {
    let right_width = width(right);
    if right_width >= max_width {
        return truncate(right, max_width);
    }
    let left = truncate(left, max_width - right_width);
    let used = width(&left);
    format!(
        "{left}{}{right}",
        " ".repeat(max_width.saturating_sub(used + right_width))
    )
}

/// Turn an arbitrary seed into a stable colour from a restrained set.
#[must_use]
pub fn seed_colour(seed: &str, accent: bool) -> ratatui::style::Color {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();
    let palettes = [
        (
            ratatui::style::Color::Rgb(76, 58, 30),
            ratatui::style::Color::Rgb(197, 150, 51),
        ),
        (
            ratatui::style::Color::Rgb(32, 57, 60),
            ratatui::style::Color::Rgb(89, 170, 156),
        ),
        (
            ratatui::style::Color::Rgb(65, 39, 47),
            ratatui::style::Color::Rgb(186, 92, 113),
        ),
        (
            ratatui::style::Color::Rgb(36, 48, 72),
            ratatui::style::Color::Rgb(112, 150, 211),
        ),
        (
            ratatui::style::Color::Rgb(53, 50, 34),
            ratatui::style::Color::Rgb(180, 173, 93),
        ),
    ];
    let (dark, bright) = palettes[(hash as usize) % palettes.len()];
    if accent {
        bright
    } else {
        dark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_respects_display_width() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(width(&fit("ab", 5)), 5);
        assert_eq!(truncate("Полярное сияние", 8), "Полярно…");
    }

    #[test]
    fn durations_and_totals_are_readable() {
        assert_eq!(duration(Duration::from_secs(107)), "1:47");
        assert_eq!(duration(Duration::from_secs(3_725)), "1:02:05");
        assert_eq!(total_time(9_840_000), "2 h 44 min");
    }

    #[test]
    fn wrapping_handles_long_cyrillic_lines() {
        let wrapped = wrap("под полярным светом город спит в снегу", 12);
        assert!(wrapped.iter().all(|line| width(line) <= 12));
        assert!(wrapped.len() > 1);
    }

    #[test]
    fn window_scroll_keeps_the_selection_visible() {
        assert_eq!(window_start(0, 20, 8), 0);
        assert_eq!(window_start(10, 20, 8), 6);
        assert_eq!(window_start(19, 20, 8), 12);
    }
}
