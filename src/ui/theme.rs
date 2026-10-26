//! The interface design system.
//!
//! A restrained black-and-yellow palette, warm off-white type, and very few
//! borders. Colour is used to signal focus and playback, not to decorate every
//! line. All colours are truecolor and degrade to the nearest terminal palette
//! on terminals without 24-bit support.

use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(9, 9, 10);
pub const PANEL: Color = Color::Rgb(14, 14, 16);
pub const PANEL_ALT: Color = Color::Rgb(20, 20, 22);
pub const SURFACE: Color = Color::Rgb(25, 25, 27);
pub const SELECT: Color = Color::Rgb(31, 30, 25);
pub const SELECT_FOCUS: Color = Color::Rgb(40, 38, 28);
pub const BORDER: Color = Color::Rgb(43, 43, 47);
pub const BORDER_SOFT: Color = Color::Rgb(31, 31, 34);
pub const TEXT: Color = Color::Rgb(244, 242, 235);
pub const TEXT_SOFT: Color = Color::Rgb(201, 198, 187);
pub const MUTED: Color = Color::Rgb(139, 138, 132);
pub const FAINT: Color = Color::Rgb(84, 84, 88);
pub const YELLOW: Color = Color::Rgb(255, 204, 0);
pub const YELLOW_SOFT: Color = Color::Rgb(255, 226, 115);
pub const YELLOW_DIM: Color = Color::Rgb(132, 108, 24);
pub const RED: Color = Color::Rgb(255, 113, 106);
pub const RED_DIM: Color = Color::Rgb(67, 31, 31);
pub const GREEN: Color = Color::Rgb(124, 217, 157);
pub const BLUE: Color = Color::Rgb(147, 183, 246);
pub const SHADOW: Color = Color::Rgb(3, 3, 4);

pub fn base() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn panel() -> Style {
    Style::default().fg(TEXT).bg(PANEL)
}

pub fn surface() -> Style {
    Style::default().fg(TEXT).bg(PANEL_ALT)
}

pub fn dim() -> Style {
    Style::default().fg(MUTED).bg(BG)
}

pub fn faint() -> Style {
    Style::default().fg(FAINT).bg(BG)
}

pub fn accent() -> Style {
    Style::default().fg(YELLOW).bg(BG)
}

pub fn bold() -> Style {
    base().add_modifier(Modifier::BOLD)
}

pub fn bold_accent() -> Style {
    accent().add_modifier(Modifier::BOLD)
}

pub fn key_style() -> Style {
    Style::default()
        .fg(YELLOW_SOFT)
        .bg(PANEL_ALT)
        .add_modifier(Modifier::BOLD)
}

pub fn subtle_key() -> Style {
    Style::default().fg(TEXT_SOFT).bg(PANEL_ALT)
}

pub fn notice_colour(kind: crate::app::NoticeKind) -> Color {
    match kind {
        crate::app::NoticeKind::Info => BLUE,
        crate::app::NoticeKind::Success => GREEN,
        crate::app::NoticeKind::Error => RED,
    }
}
