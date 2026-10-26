//! The dedicated Now Playing screen.
//!
//! Large generated cover art on the left, the current identity and queue context
//! below it, and the full lyric sheet on the right. Time-coded lyrics follow
//! playback automatically; `j`/`k` in the lyric pane nudges the scroll position
//! without leaving the screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Mode};
use crate::ui::{components, cover, icons, primitives, text, theme};

/// Draw the full Now Playing experience in the middle of the application.
pub fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width < 30 || area.height < 8 {
        return;
    }
    primitives::paint(frame, area, theme::base());
    let Some(track) = &app.current else {
        components::message_panel(
            frame,
            area,
            icons::HEADPHONES,
            "Nothing playing",
            "Choose a track from your library and press Enter.",
            theme::MUTED,
        );
        return;
    };

    let left_width = if area.width >= 100 {
        (area.width * 45 / 100).max(36)
    } else {
        (area.width * 44 / 100).max(28)
    }
    .min(area.width.saturating_sub(28));
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_width), Constraint::Min(20)])
        .split(area);
    let left = columns[0];
    let right = columns[1];

    draw_identity(frame, app, left);
    draw_lyrics(frame, app, right);

    let _ = track;
}

fn draw_identity(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let pad = if area.width >= 40 { 3 } else { 2 };
    let content_width = area.width.saturating_sub(pad * 2);
    // Metadata is more useful than another row of cover art. Reserve ten rows
    // for the identity block so title, artist, album, badges, and "up next"
    // survive on an 80×24 terminal instead of being clipped by a square cover.
    let available = area.height.saturating_sub(10);
    let natural_height = content_width / 2;
    let cover_height = natural_height.min(available);
    let cover_area = Rect::new(area.x + pad, area.y + 1, content_width, cover_height);
    if cover_area.width >= 8 && cover_area.height >= 4 {
        if let Some(image) = app.cover.as_ref().filter(|_| app.show_covers) {
            cover::draw_cover(frame, cover_area, image);
        } else if app.current.is_some() {
            let seed = app
                .current
                .as_ref()
                .map(|track| format!("{} / {}", track.title, track.artist))
                .unwrap_or_else(|| "yandex music".into());
            cover::draw_placeholder(frame, cover_area, &seed, app.mode.label());
        }
    }

    let mut y = cover_area.y + cover_area.height + 1;
    if y >= area.y + area.height {
        return;
    }
    let inner_x = area.x + pad;
    let title_width = usize::from(content_width);
    let Some(track) = &app.current else {
        return;
    };

    components::eyebrow(
        frame,
        Rect::new(inner_x, y, content_width, 1),
        "NOW PLAYING",
    );
    if area.width >= 40 {
        let track_no = app.queue_cursor + 1;
        let total = app.queue.len();
        primitives::right_line(
            frame,
            Rect::new(inner_x, y, content_width, 1),
            vec![Span::styled(
                format!("{track_no:02} / {total:02}"),
                Style::default().fg(theme::FAINT),
            )],
            theme::base(),
        );
    }
    y += 2;
    if y >= area.y + area.height {
        return;
    }
    primitives::line(
        frame,
        Rect::new(inner_x, y, content_width, 1),
        vec![Span::styled(
            text::truncate(&track.title, title_width),
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::BG)
                .add_modifier(Modifier::BOLD),
        )],
        theme::base(),
    );
    y += 1;
    if y >= area.y + area.height {
        return;
    }
    primitives::line(
        frame,
        Rect::new(inner_x, y, content_width, 1),
        vec![Span::styled(
            text::truncate(&track.artist, title_width.saturating_sub(2)),
            Style::default().fg(theme::YELLOW_SOFT).bg(theme::BG),
        )],
        theme::base(),
    );
    y += 1;
    if y < area.y + area.height {
        let mut meta = track.album.clone().unwrap_or_default();
        if let Some(year) = track.year {
            if !meta.is_empty() {
                meta.push_str(" · ");
            }
            meta.push_str(&year.to_string());
        }
        if meta.is_empty() {
            meta = "Yandex Music".into();
        }
        primitives::line(
            frame,
            Rect::new(inner_x, y, content_width, 1),
            vec![Span::styled(
                text::truncate(&meta, title_width),
                Style::default().fg(theme::MUTED).bg(theme::BG),
            )],
            theme::base(),
        );
        y += 1;
    }

    if y < area.y + area.height {
        let mut badges: Vec<Span<'static>> = Vec::new();
        if app.mode == Mode::Demo {
            badges.push(components::state_badge("SIMULATED", theme::YELLOW_DIM));
        }
        if app.liked_ids.contains(&track.id) {
            badges.push(Span::styled(" ", Style::default().bg(theme::BG)));
            badges.push(components::state_badge("LIKED", theme::YELLOW_SOFT));
        }
        if app.cached_ids.contains(&track.id) {
            badges.push(Span::styled(" ", Style::default().bg(theme::BG)));
            badges.push(components::state_badge("OFFLINE", theme::GREEN));
        }
        if track.explicit {
            badges.push(Span::styled(" ", Style::default().bg(theme::BG)));
            badges.push(components::state_badge("EXPLICIT", theme::RED));
        }
        primitives::line(
            frame,
            Rect::new(inner_x, y, content_width, 1),
            badges,
            theme::base(),
        );
        y += 1;
    }

    if y + 1 < area.y + area.height {
        if let Some(next) = app.queue.get(app.queue_cursor + 1) {
            y += 1;
            if y < area.y + area.height {
                primitives::line(
                    frame,
                    Rect::new(inner_x, y, content_width, 1),
                    vec![
                        Span::styled("UP NEXT  ", Style::default().fg(theme::FAINT)),
                        Span::styled(
                            text::truncate(&next.title, title_width.saturating_sub(9)),
                            Style::default().fg(theme::TEXT_SOFT),
                        ),
                    ],
                    theme::base(),
                );
            }
            y += 1;
        }
        if y < area.y + area.height && app.lyrics_offset != 0 {
            primitives::line(
                frame,
                Rect::new(inner_x, y, content_width, 1),
                vec![Span::styled(
                    "lyrics scrolled · press g to follow again",
                    Style::default().fg(theme::FAINT),
                )],
                theme::base(),
            );
        }
    }
}

fn draw_lyrics(frame: &mut Frame<'_>, app: &App, area: Rect) {
    primitives::paint(frame, area, theme::panel());
    primitives::vertical_rule(
        frame,
        area.x,
        area.y,
        area.height,
        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL),
    );
    let pad = if area.width >= 34 { 3 } else { 2 };
    let inner = Rect::new(
        area.x + pad,
        area.y,
        area.width.saturating_sub(pad + 2),
        area.height,
    );
    if inner.width < 6 || inner.height < 4 {
        return;
    }

    let heading_y = area.y + 1;
    components::eyebrow(
        frame,
        Rect::new(inner.x, heading_y, inner.width, 1),
        "LYRICS",
    );
    let tag = if app.lyrics_loading {
        format!("{} FINDING", components::spinner(app.animation_tick))
    } else if app.lyrics.as_ref().is_some_and(|lyrics| lyrics.synced) {
        format!("{} SYNced", icons::CHECK).to_uppercase()
    } else {
        "PLAIN TEXT".to_owned()
    };
    primitives::right_line(
        frame,
        Rect::new(inner.x, heading_y, inner.width, 1),
        vec![Span::styled(tag, Style::default().fg(theme::FAINT))],
        theme::base(),
    );
    primitives::rule(
        frame,
        Rect::new(inner.x, heading_y + 1, inner.width, 1),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL),
    );
    let body = Rect::new(
        inner.x,
        heading_y + 3,
        inner.width,
        area.y + area.height - (heading_y + 3),
    );
    if body.height == 0 {
        return;
    }

    if app.lyrics_loading {
        lyric_message(
            frame,
            body,
            "Finding the lyric sheet",
            "Yandex Music is answering in the background.",
            components::spinner(app.animation_tick),
        );
        return;
    }
    if let Some(error) = &app.lyrics_error {
        lyric_message(frame, body, "Lyrics are unavailable", error, icons::WARNING);
        return;
    }
    let Some(lyrics) = &app.lyrics else {
        lyric_message(
            frame,
            body,
            "An instrumental",
            "No lyric sheet was published for this track.",
            icons::LYRICS,
        );
        return;
    };
    if lyrics.lines.is_empty() {
        lyric_message(
            frame,
            body,
            "An instrumental",
            "No lyric sheet was published for this track.",
            icons::LYRICS,
        );
        return;
    }

    let text_width = usize::from(body.width).saturating_sub(2);
    if text_width < 4 {
        return;
    }
    let active = lyrics
        .active_index(app.position, app.duration)
        .unwrap_or(0)
        .min(lyrics.lines.len().saturating_sub(1));
    let mut rows: Vec<(usize, String)> = Vec::new();
    for (index, line) in lyrics.lines.iter().enumerate() {
        let wrapped = text::wrap(&line.text, text_width);
        if wrapped.is_empty() {
            rows.push((index, String::new()));
        } else {
            for part in wrapped {
                rows.push((index, part));
            }
        }
    }
    let active_visual = rows
        .iter()
        .position(|(index, _)| *index == active)
        .unwrap_or(0);
    let target = active_visual as i64 + i64::from(app.lyrics_offset);
    // Keep the final row free for the source badge, which is right-aligned in
    // the same space as the lyric sheet on short terminals.
    let visible = body.height.saturating_sub(u16::from(body.height >= 6));
    let start = (target - i64::from(body.height / 2)).max(0) as usize;
    let start = start.min(rows.len().saturating_sub(usize::from(visible.max(1))));

    for (y, (visual, (_, line))) in (body.y..).zip(
        rows.iter()
            .enumerate()
            .skip(start)
            .take(usize::from(visible)),
    ) {
        let is_active = visual == active_visual;
        let distance = visual.abs_diff(active_visual);
        let style = if is_active {
            Style::default()
                .fg(theme::YELLOW_SOFT)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD)
        } else if distance <= 2 {
            Style::default().fg(theme::TEXT_SOFT).bg(theme::PANEL)
        } else if distance <= 6 {
            Style::default().fg(theme::MUTED).bg(theme::PANEL)
        } else {
            Style::default().fg(theme::FAINT).bg(theme::PANEL)
        };
        primitives::line(
            frame,
            Rect::new(body.x, y, body.width, 1),
            vec![
                Span::styled(
                    if is_active { "▌ " } else { "  " },
                    Style::default()
                        .fg(if is_active {
                            theme::YELLOW
                        } else {
                            theme::PANEL
                        })
                        .bg(theme::PANEL),
                ),
                Span::styled(line.clone(), style),
            ],
            Style::default().bg(theme::PANEL),
        );
    }

    if body.height >= 6 {
        primitives::right_line(
            frame,
            Rect::new(body.x, body.y + body.height - 1, body.width, 1),
            vec![Span::styled(
                if app.lyrics_offset == 0 {
                    format!("{} TIME-SYNCED", icons::DOT)
                } else {
                    format!("{} SCROLLED +{}", icons::DOT, app.lyrics_offset)
                },
                Style::default().fg(theme::FAINT).bg(theme::PANEL),
            )],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn lyric_message(frame: &mut Frame<'_>, area: Rect, title: &str, message: &str, icon: &str) {
    let lines = vec![
        Line::from(Span::styled(
            icon.to_owned(),
            Style::default().fg(theme::FAINT),
        )),
        Line::from(""),
        Line::from(Span::styled(
            title.to_owned(),
            Style::default()
                .fg(theme::TEXT_SOFT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            message.to_owned(),
            Style::default().fg(theme::MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(theme::PANEL)),
        area,
    );
}
