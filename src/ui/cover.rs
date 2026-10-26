//! Truecolor half-block cover art and its terminal-native fallbacks.
//!
//! The image path uses the upper-half block (`▀`): the foreground is the top
//! pixel and the background is the bottom pixel. That yields square pixels in a
//! normal 1:2 terminal cell and needs no image protocol.

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::cover::CoverImage;
use crate::ui::text::seed_colour;
use crate::ui::theme;

/// Paint a square cover centred inside `area`.
pub fn draw_cover(frame: &mut Frame<'_>, area: Rect, image: &CoverImage) {
    if area.width < 2 || area.height < 1 || image.width == 0 || image.height == 0 {
        return;
    }

    // A cell is roughly twice as tall as it is wide, so a square image needs a
    // 2:1 (width:height) cell box.
    let mut width = area.width.min(area.height.saturating_mul(2));
    if width < 2 {
        width = 2;
    }
    let height = (width / 2).min(area.height);
    width = (height * 2).min(area.width);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let target = Rect::new(x, y, width, height);

    let side = image.width.min(image.height);
    let source_x = (image.width - side) / 2;
    let source_y = (image.height - side) / 2;
    let buffer = frame.buffer_mut();
    for row in 0..target.height {
        for col in 0..target.width {
            let top_u = (u32::from(col) * side) / u32::from(target.width.max(1));
            let bottom_u = (u32::from(col) * side) / u32::from(target.width.max(1));
            let top_v = ((u32::from(row) * 2 * side) / u32::from((target.height * 2).max(1)))
                .min(side.saturating_sub(1));
            let bottom_v = (((u32::from(row) * 2 + 1) * side)
                / u32::from((target.height * 2).max(1)))
            .min(side.saturating_sub(1));
            let top = image.sample(
                source_x + top_u.min(side.saturating_sub(1)),
                source_y + top_v,
            );
            let bottom = image.sample(
                source_x + bottom_u.min(side.saturating_sub(1)),
                source_y + bottom_v,
            );
            if let Some(cell) = buffer.cell_mut((target.x + col, target.y + row)) {
                cell.set_symbol("▀")
                    .set_fg(Color::Rgb(top[0], top[1], top[2]))
                    .set_bg(Color::Rgb(bottom[0], bottom[1], bottom[2]));
            }
        }
    }
}

/// A designed, non-photographic cover when artwork is disabled or absent.
pub fn draw_placeholder(frame: &mut Frame<'_>, area: Rect, seed: &str, message: &str) {
    if area.width < 8 || area.height < 4 {
        return;
    }
    let width = area.width.min(area.height.saturating_mul(2));
    let height = (width / 2).min(area.height);
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let shade = Style::default()
        .fg(seed_colour(seed, true))
        .bg(seed_colour(seed, false));
    frame.render_widget(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .style(Style::default().bg(theme::PANEL_ALT)),
        box_area,
    );
    let inner = box_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.height < 2 || inner.width < 4 {
        return;
    }
    // A crosshatch of blocks gives the placeholder some material without
    // pretending to be album art.
    let mut lines = Vec::new();
    let hatch = "▚";
    for row in 0..inner.height.saturating_sub(2) {
        let mut line = String::new();
        for col in 0..inner.width {
            if (row + col) % 5 == 0 {
                line.push_str(hatch);
            } else {
                line.push(' ');
            }
        }
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            line, shade,
        )));
    }
    let label = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled("♪  ", theme::bold_accent()),
        ratatui::text::Span::styled(message, theme::bold()),
    ]);
    lines.insert(lines.len() / 2, label);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().bg(theme::PANEL_ALT)),
        inner,
    );
}

/// A compact album tile used by playlist/album/artist rows. The tile is two rows
/// high and up to four columns wide, which makes it physically square.
pub fn draw_tile(frame: &mut Frame<'_>, area: Rect, seed: &str, selected: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let bright = seed_colour(seed, true);
    let dark = seed_colour(seed, false);
    let buffer = frame.buffer_mut();
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(cell) = buffer.cell_mut((area.x + x, area.y + y)) {
                let blend = if (x as usize + y as usize * 2) % 3 == 0 {
                    bright
                } else {
                    dark
                };
                cell.set_symbol(if selected { "▚" } else { "▀" })
                    .set_fg(blend)
                    .set_bg(if selected {
                        theme::SELECT_FOCUS
                    } else {
                        theme::PANEL_ALT
                    });
            }
        }
    }
    if area.width >= 3 && area.height >= 2 {
        if let Some(centre) = buffer.cell_mut((area.x + area.width / 2, area.y + area.height / 2)) {
            centre.set_symbol("♪").set_fg(theme::YELLOW_SOFT);
        }
    }
}

/// Fill a rectangular area with a soft generated atmosphere for the auth screen.
pub fn draw_atmosphere(frame: &mut Frame<'_>, area: Rect, seed: &str) {
    if area.width < 4 || area.height < 4 {
        return;
    }
    let image = CoverImage::procedural(seed);
    draw_cover(frame, area, &image);
    // Letterbox with a darker matte so the image floats on the auth layout.
    let left = area.x;
    let right = area.x + area.width.saturating_sub(1);
    let buffer = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        if let Some(cell) = buffer.cell_mut((left, y)) {
            cell.set_symbol(" ");
            cell.set_bg(theme::BG);
        }
        if let Some(cell) = buffer.cell_mut((right, y)) {
            cell.set_symbol(" ");
            cell.set_bg(theme::BG);
        }
    }
}
