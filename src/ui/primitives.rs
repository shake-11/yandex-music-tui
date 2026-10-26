//! Low-level rendering helpers shared by every screen.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::ui::text::width;

/// Fill an area with a style.
pub fn paint(frame: &mut Frame<'_>, area: Rect, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.buffer_mut().set_style(area, style);
}

/// Draw one line of styled spans, clipped to `area`.
pub fn line(frame: &mut Frame<'_>, area: Rect, spans: Vec<Span<'_>>, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.buffer_mut().set_style(area, style);
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
}

/// Draw a plain string.
pub fn text(frame: &mut Frame<'_>, area: Rect, value: impl Into<String>, style: Style) {
    line(frame, area, vec![Span::styled(value.into(), style)], style);
}

/// Draw right-aligned styled spans.
pub fn right_line(frame: &mut Frame<'_>, area: Rect, spans: Vec<Span<'_>>, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.buffer_mut().set_style(area, style);
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .style(style)
            .alignment(Alignment::Right),
        area,
    );
}

/// A faint horizontal rule.
pub fn rule(frame: &mut Frame<'_>, area: Rect, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let y = area.y;
    let buffer = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        if let Some(cell) = buffer.cell_mut((x, y)) {
            cell.set_symbol("─").set_style(style);
        }
    }
}

/// A vertical rule.
pub fn vertical_rule(frame: &mut Frame<'_>, x: u16, y: u16, height: u16, style: Style) {
    let buffer = frame.buffer_mut();
    for row in y..y.saturating_add(height) {
        if let Some(cell) = buffer.cell_mut((x, row)) {
            cell.set_symbol("│").set_style(style);
        }
    }
}

/// Sum of rendered span widths, useful for right-aligned chrome.
pub fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| width(&span.content)).sum()
}

/// Pad a vector of spans with spaces until it is `total` columns wide.
pub fn pad_spans(spans: &mut Vec<Span<'static>>, total: usize, style: Style) {
    let used = spans_width(spans);
    if used < total {
        spans.push(Span::styled(" ".repeat(total - used), style));
    }
}
