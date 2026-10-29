//! Persistent chrome: header, sidebar, player transport, and status footer.
//!
//! These components are deliberately ignorant of the middle-pane content. They
//! only read the small part of `App` they need, which keeps the layout stable
//! while screens change and makes the compact breakpoints explicit.

use std::time::Duration;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Focus, Mode, Overlay, Section, SEARCH_NAV_INDEX};
use crate::ui::text::{self, fit, truncate, width};
use crate::ui::{icons, primitives, theme};

const NAV_ROWS: usize = Section::ALL.len() + 1;

/// Two-line application header.
pub fn header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 || area.width < 20 {
        return;
    }
    primitives::paint(frame, area, theme::base());
    let top = Rect::new(area.x, area.y, area.width, 1);
    let bottom = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        1,
    );

    let compact = area.width < 72;
    let brand = brand_spans(compact);
    let profile = profile_spans(app, area.width);
    let profile_width = primitives::spans_width(&profile)
        .min(usize::from(area.width / 2))
        .max(1) as u16;
    let brand_area = Rect::new(
        area.x + 2,
        top.y,
        area.width.saturating_sub(profile_width + 6),
        1,
    );
    primitives::line(frame, brand_area, brand, theme::base());
    primitives::right_line(
        frame,
        Rect::new(
            area.x + area.width.saturating_sub(profile_width + 2),
            top.y,
            profile_width,
            1,
        ),
        profile,
        theme::base(),
    );

    if area.height >= 2 {
        let status = sync_status(app);
        let status_width = (width(&status) + 2).min(usize::from(area.width / 2)).max(1) as u16;
        let crumbs_width = area.width.saturating_sub(status_width + 7);
        primitives::line(
            frame,
            Rect::new(area.x + 2, bottom.y, crumbs_width, 1),
            breadcrumbs(app, usize::from(crumbs_width)),
            theme::base(),
        );
        primitives::right_line(
            frame,
            Rect::new(
                area.x + area.width.saturating_sub(status_width + 2),
                bottom.y,
                status_width,
                1,
            ),
            vec![Span::styled(status, status_style(app))],
            theme::base(),
        );
    }
}

fn brand_spans(compact: bool) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            "▰ ",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if compact { "YM" } else { "YANDEX" },
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if compact { " / M" } else { " / MUSIC" },
            Style::default().fg(theme::MUTED),
        ),
        Span::styled(
            " TUI ",
            Style::default()
                .fg(theme::YELLOW_SOFT)
                .bg(theme::SURFACE)
                .add_modifier(Modifier::BOLD),
        ),
    ]
}

fn profile_spans(app: &App, total_width: u16) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!(" {} ", app.mode.label()),
        mode_badge(app.mode),
    ));
    if app.signed_in {
        let name_width = if total_width >= 120 {
            18
        } else if total_width >= 88 {
            12
        } else {
            8
        };
        spans.push(Span::styled(" ", theme::base()));
        spans.push(Span::styled(
            icons::ARTIST,
            Style::default().fg(theme::FAINT),
        ));
        spans.push(Span::styled(
            format!(" {}", truncate(app.profile.short_name(), name_width)),
            Style::default().fg(theme::TEXT_SOFT),
        ));
        if app.profile.has_plus && total_width >= 88 {
            spans.push(Span::styled(" ", theme::base()));
            spans.push(Span::styled(
                " PLUS ",
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::SURFACE)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }
    spans
}

fn breadcrumbs(app: &App, max_width: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::styled(
        "LIBRARY",
        Style::default()
            .fg(theme::YELLOW_DIM)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled("  /  ", Style::default().fg(theme::FAINT)));
    spans.push(Span::styled(
        app.section.title().to_uppercase(),
        Style::default().fg(theme::TEXT_SOFT),
    ));
    if app.now_playing {
        spans.push(Span::styled("  /  ", Style::default().fg(theme::FAINT)));
        spans.push(Span::styled(
            "NOW PLAYING",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        ));
    } else if let crate::app::View::Collection(collection) = &app.view {
        let used = primitives::spans_width(&spans);
        let room = max_width.saturating_sub(used + 5);
        if room > 4 {
            spans.push(Span::styled("  /  ", Style::default().fg(theme::FAINT)));
            spans.push(Span::styled(
                truncate(&collection.request.title(), room).to_uppercase(),
                Style::default().fg(theme::TEXT_SOFT),
            ));
        }
    }
    spans
}

fn mode_badge(mode: Mode) -> Style {
    match mode {
        Mode::Live => Style::default()
            .fg(theme::YELLOW_SOFT)
            .bg(theme::SURFACE)
            .add_modifier(Modifier::BOLD),
        Mode::Demo => Style::default()
            .fg(theme::BG)
            .bg(theme::YELLOW)
            .add_modifier(Modifier::BOLD),
    }
}

fn sync_status(app: &App) -> String {
    if app.retry_status().is_some() {
        format!("{}  RETRYING", spinner(app.animation_tick))
    } else if app.library_error.is_some() && app.library.is_none() {
        format!("{}  OFFLINE", icons::WARNING)
    } else if app.library_loading {
        format!("{}  SYNCING", spinner(app.animation_tick))
    } else if app.wave.as_ref().is_some_and(|wave| wave.loading) {
        format!("{}  TUNING WAVE", spinner(app.animation_tick))
    } else if app.mode == Mode::Demo {
        format!("{}  SIMULATED", icons::DOT)
    } else {
        format!("{}  IN SYNC", icons::DOT)
    }
}

fn status_style(app: &App) -> Style {
    if app.retry_status().is_some()
        || app.library_loading
        || app.wave.as_ref().is_some_and(|wave| wave.loading)
    {
        Style::default().fg(theme::YELLOW_SOFT)
    } else if app.library_error.is_some() && app.library.is_none() {
        Style::default().fg(theme::RED)
    } else if app.mode == Mode::Demo {
        Style::default().fg(theme::YELLOW_DIM)
    } else {
        Style::default().fg(theme::GREEN)
    }
}

/// Sidebar navigation, or the seven-column icon rail on narrow terminals.
pub fn sidebar(frame: &mut Frame<'_>, app: &App, area: Rect, compact: bool) {
    if area.width < 3 || area.height < 3 {
        return;
    }
    primitives::paint(frame, area, theme::panel());
    let focused = app.focus == Focus::Sidebar;
    let border_style = if focused {
        Style::default().fg(theme::YELLOW_DIM)
    } else {
        Style::default().fg(theme::BORDER_SOFT)
    };
    primitives::vertical_rule(
        frame,
        area.x + area.width - 1,
        area.y,
        area.height,
        border_style,
    );

    if compact || area.width < 20 {
        compact_sidebar(frame, app, area);
        return;
    }

    let inner_x = area.x + 2;
    let inner_width = area.width.saturating_sub(4);
    let mut y = area.y + 2;
    primitives::line(
        frame,
        Rect::new(inner_x, y, inner_width, 1),
        vec![
            Span::styled(
                "LIBRARY",
                Style::default()
                    .fg(theme::FAINT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", icons::DOT),
                Style::default().fg(theme::YELLOW_DIM),
            ),
        ],
        theme::panel(),
    );
    y += 2;

    for index in 0..NAV_ROWS {
        if index == SEARCH_NAV_INDEX {
            if y + 2 > area.y + area.height {
                break;
            }
            primitives::rule(
                frame,
                Rect::new(inner_x, y, inner_width, 1),
                Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL),
            );
            y += 1;
        }
        if y >= area.y + area.height {
            break;
        }
        let (icon, label, shortcut) = if index == SEARCH_NAV_INDEX {
            (icons::SEARCH, "Search", "/".to_owned())
        } else {
            let section = Section::ALL[index];
            (
                section.icon(),
                section.title(),
                section.shortcut().to_string(),
            )
        };
        draw_nav_row(
            frame,
            app,
            area,
            inner_x,
            inner_width,
            y,
            index,
            icon,
            label,
            &shortcut,
        );
        y += 1;
    }

    // Bottom cache card, only when there is room for it to breathe.
    if area.height >= 26 {
        let card_y = area.y + area.height - 6;
        if card_y > y + 1 {
            primitives::rule(
                frame,
                Rect::new(inner_x, card_y, inner_width, 1),
                Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL),
            );
            primitives::line(
                frame,
                Rect::new(inner_x, card_y + 1, inner_width, 1),
                vec![Span::styled(
                    "ON THIS DEVICE",
                    Style::default()
                        .fg(theme::FAINT)
                        .add_modifier(Modifier::BOLD),
                )],
                theme::panel(),
            );
            primitives::line(
                frame,
                Rect::new(inner_x, card_y + 2, inner_width, 1),
                vec![
                    Span::styled(
                        format!("{} ", icons::DOWNLOAD),
                        Style::default().fg(theme::YELLOW_DIM),
                    ),
                    Span::styled(
                        app.cached_count.to_string(),
                        Style::default()
                            .fg(theme::TEXT_SOFT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" tracks cached", Style::default().fg(theme::MUTED)),
                ],
                theme::panel(),
            );
            let summary = match (&app.library, app.mode) {
                (Some(library), Mode::Demo) => format!(
                    "{} liked · {} collections",
                    library.liked_tracks.len(),
                    library.playlists.len()
                ),
                (Some(library), Mode::Live) => format!(
                    "{} liked · {} playlists",
                    library.liked_tracks.len(),
                    library.playlists.len()
                ),
                (None, _) if app.library_loading => "Building your library…".into(),
                (None, _) => "Waiting for your library".into(),
            };
            primitives::line(
                frame,
                Rect::new(inner_x, card_y + 3, inner_width, 1),
                vec![Span::styled(
                    truncate(&summary, usize::from(inner_width)),
                    Style::default().fg(theme::MUTED),
                )],
                theme::panel(),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_nav_row(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    inner_x: u16,
    inner_width: u16,
    y: u16,
    index: usize,
    icon: &str,
    label: &str,
    shortcut: &str,
) {
    let selected = app.nav_index == index;
    let focused = app.focus == Focus::Sidebar;
    let row = Rect::new(inner_x, y, inner_width, 1);
    let background = if selected {
        if focused {
            theme::SELECT_FOCUS
        } else {
            theme::SELECT
        }
    } else {
        theme::PANEL
    };
    if row.y >= area.y + area.height {
        return;
    }
    primitives::paint(frame, row, Style::default().bg(background).fg(theme::TEXT));

    let marker = Span::styled(
        "▌",
        Style::default()
            .fg(if selected {
                if focused {
                    theme::YELLOW
                } else {
                    theme::YELLOW_DIM
                }
            } else {
                background
            })
            .bg(background),
    );
    let icon_style = Style::default()
        .fg(if selected {
            if focused {
                theme::YELLOW
            } else {
                theme::YELLOW_SOFT
            }
        } else {
            theme::FAINT
        })
        .bg(background);
    let label_style = Style::default()
        .fg(if selected {
            if focused {
                theme::TEXT
            } else {
                theme::TEXT_SOFT
            }
        } else {
            theme::MUTED
        })
        .bg(background)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    let shortcut_style = Style::default()
        .fg(if selected {
            theme::YELLOW_DIM
        } else {
            theme::FAINT
        })
        .bg(background);

    let shortcut_width = width(shortcut);
    let fixed = 2 + width(icon) + 2 + shortcut_width + 2;
    let label_room = usize::from(inner_width).saturating_sub(fixed);
    let mut spans = vec![
        marker,
        Span::styled(" ", Style::default().bg(background)),
        Span::styled(format!("{icon}  "), icon_style),
        Span::styled(truncate(label, label_room), label_style),
    ];
    let pad = usize::from(inner_width)
        .saturating_sub(primitives::spans_width(&spans) + shortcut_width + 1);
    spans.push(Span::styled(
        " ".repeat(pad),
        Style::default().bg(background),
    ));
    spans.push(Span::styled(shortcut.to_owned(), shortcut_style));
    spans.push(Span::styled(" ", Style::default().bg(background)));
    primitives::line(frame, row, spans, Style::default().bg(background));
}

fn compact_sidebar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let inner_width = area.width.saturating_sub(1);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "▰",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .style(theme::panel()),
        Rect::new(area.x, area.y + 1, inner_width, 1),
    );

    let item_height = 2u16;
    let nav_start = area.y + 3;
    for index in 0..NAV_ROWS {
        let y = nav_start + index as u16 * item_height + u16::from(index == SEARCH_NAV_INDEX);
        if y + item_height > area.y + area.height {
            break;
        }
        let selected = app.nav_index == index;
        let focused = app.focus == Focus::Sidebar;
        let background = if selected {
            if focused {
                theme::SELECT_FOCUS
            } else {
                theme::SELECT
            }
        } else {
            theme::PANEL
        };
        let row = Rect::new(area.x, y, inner_width, item_height);
        primitives::paint(frame, row, Style::default().bg(background));
        if selected {
            primitives::vertical_rule(
                frame,
                area.x,
                y,
                item_height,
                Style::default()
                    .fg(if focused {
                        theme::YELLOW
                    } else {
                        theme::YELLOW_DIM
                    })
                    .bg(background),
            );
        }
        let icon = if index == SEARCH_NAV_INDEX {
            icons::SEARCH
        } else {
            Section::ALL[index].icon()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                icon,
                Style::default()
                    .fg(if selected {
                        if focused {
                            theme::YELLOW
                        } else {
                            theme::YELLOW_SOFT
                        }
                    } else {
                        theme::MUTED
                    })
                    .bg(background),
            )))
            .alignment(Alignment::Center)
            .style(Style::default().bg(background)),
            Rect::new(area.x + 1, y, inner_width.saturating_sub(1), item_height),
        );
    }

    if area.height >= 30 {
        let y = area.y + area.height - 3;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{} {}", icons::DOWNLOAD, app.cached_count),
                Style::default().fg(theme::FAINT).bg(theme::PANEL),
            )))
            .alignment(Alignment::Center)
            .style(Style::default().bg(theme::PANEL)),
            Rect::new(area.x, y, inner_width, 1),
        );
    }
}

/// Player transport: now-playing metadata, controls, progress, and volume.
pub fn player_bar(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    primitives::paint(frame, area, theme::panel());
    if area.height == 1 {
        draw_transport(frame, app, area);
        return;
    }
    primitives::rule(
        frame,
        Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), 1),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::BG),
    );
    let content = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(content);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(44),
            Constraint::Percentage(22),
        ])
        .split(rows[0]);

    draw_now_playing_summary(
        frame,
        app,
        Rect::new(
            columns[0].x + 2,
            rows[0].y,
            columns[0].width.saturating_sub(4),
            1,
        ),
    );

    let center = Rect::new(
        columns[1].x + 1,
        rows[0].y,
        columns[1].width.saturating_sub(2),
        1,
    );
    draw_transport(frame, app, center);
    let second = Rect::new(
        columns[1].x + 1,
        rows[1].y,
        columns[1].width.saturating_sub(2),
        1,
    );
    draw_progress(frame, app, second);

    let right = Rect::new(
        columns[2].x + 1,
        rows[0].y,
        columns[2].width.saturating_sub(3),
        rows[0].height,
    );
    draw_volume(frame, app, right);
    let source = playback_source(app);
    primitives::right_line(
        frame,
        Rect::new(
            columns[2].x + 1,
            rows[1].y,
            columns[2].width.saturating_sub(3),
            1,
        ),
        vec![Span::styled(
            source,
            Style::default()
                .fg(if app.buffering {
                    theme::YELLOW
                } else {
                    theme::FAINT
                })
                .bg(theme::PANEL),
        )],
        Style::default().bg(theme::PANEL),
    );
}

fn draw_now_playing_summary(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let marker = if app.buffering {
        Span::styled(
            format!("{} ", spinner(app.animation_tick)),
            Style::default().fg(theme::YELLOW),
        )
    } else if app.playing {
        Span::styled(
            format!(
                "{} ",
                icons::EQUALISER[(app.animation_tick as usize / 2) % 4]
            ),
            Style::default().fg(theme::YELLOW),
        )
    } else {
        Span::styled("▶ ", Style::default().fg(theme::FAINT))
    };
    let flash = app.animations && app.last_track_change.elapsed() < Duration::from_millis(900);
    if let Some(track) = &app.current {
        let title_style = Style::default()
            .fg(if flash {
                theme::YELLOW_SOFT
            } else {
                theme::TEXT
            })
            .bg(theme::PANEL)
            .add_modifier(Modifier::BOLD);
        let title = truncate(&track.title, usize::from(area.width).saturating_sub(4));
        primitives::line(
            frame,
            area,
            vec![marker, Span::styled(title, title_style)],
            Style::default().bg(theme::PANEL),
        );
    } else {
        primitives::line(
            frame,
            area,
            vec![
                marker,
                Span::styled(
                    "Nothing playing",
                    Style::default()
                        .fg(theme::TEXT_SOFT)
                        .bg(theme::PANEL)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn draw_transport(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let icon_width = width(icons::PREVIOUS).max(1);
    let button_icon = if app.buffering {
        spinner(app.animation_tick)
    } else if app.playing {
        icons::PAUSE
    } else {
        icons::PLAY
    };
    let button_style = if app.current.is_some() {
        Style::default()
            .fg(theme::BG)
            .bg(theme::YELLOW)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FAINT).bg(theme::SURFACE)
    };
    let control_style = Style::default()
        .fg(if app.current.is_some() {
            theme::TEXT_SOFT
        } else {
            theme::FAINT
        })
        .bg(theme::PANEL);
    let gap = 3usize;
    let group_width = icon_width * 2 + (width(button_icon) + 2) + gap * 2;
    let offset = usize::from(area.width).saturating_sub(group_width) / 2;
    let mut spans = Vec::new();
    if offset > 0 {
        spans.push(Span::styled(
            " ".repeat(offset),
            Style::default().bg(theme::PANEL),
        ));
    }
    spans.push(Span::styled(icons::PREVIOUS, control_style));
    spans.push(Span::styled(
        " ".repeat(gap),
        Style::default().bg(theme::PANEL),
    ));
    spans.push(Span::styled(format!(" {button_icon} "), button_style));
    spans.push(Span::styled(
        " ".repeat(gap),
        Style::default().bg(theme::PANEL),
    ));
    spans.push(Span::styled(icons::NEXT, control_style));
    primitives::line(frame, area, spans, Style::default().bg(theme::PANEL));
}

fn draw_progress(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 || area.width < 14 {
        return;
    }
    let elapsed = text::duration(app.position);
    let total = text::duration(app.duration);
    let elapsed_width = 5u16;
    let total_width = 5u16;
    let bar_width = area.width.saturating_sub(elapsed_width + total_width + 2);
    if bar_width < 2 {
        return;
    }
    let style = Style::default().bg(theme::PANEL);
    primitives::text(
        frame,
        Rect::new(area.x, area.y, elapsed_width, 1),
        elapsed,
        Style::default().fg(theme::MUTED).bg(theme::PANEL),
    );
    primitives::right_line(
        frame,
        Rect::new(area.x + area.width - total_width, area.y, total_width, 1),
        vec![Span::styled(
            total,
            Style::default().fg(theme::MUTED).bg(theme::PANEL),
        )],
        style,
    );

    let bar_area = Rect::new(area.x + elapsed_width + 1, area.y, bar_width, 1);
    let ratio = text::fraction(app.position, app.duration);
    let filled = (ratio * f64::from(bar_width.saturating_sub(1))).round() as u16;
    let buffer = frame.buffer_mut();
    for offset in 0..bar_width {
        if let Some(cell) = buffer.cell_mut((bar_area.x + offset, bar_area.y)) {
            if offset < filled {
                cell.set_symbol("━").set_fg(theme::YELLOW);
            } else if offset == filled {
                cell.set_symbol("●").set_fg(if app.current.is_some() {
                    theme::YELLOW_SOFT
                } else {
                    theme::FAINT
                });
            } else {
                cell.set_symbol("─").set_fg(theme::FAINT);
            }
            cell.set_bg(theme::PANEL);
        }
    }
}

fn draw_volume(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width < 8 || area.height == 0 {
        return;
    }
    let effective = if app.muted { 0.0 } else { app.volume };
    let cells: usize = if area.width >= 22 { 6 } else { 4 };
    let filled = (effective * cells as f32).round() as usize;
    let mut spans = vec![Span::styled(
        if app.muted {
            icons::MUTED
        } else {
            icons::VOLUME
        },
        Style::default()
            .fg(if app.muted { theme::RED } else { theme::MUTED })
            .bg(theme::PANEL),
    )];
    spans.push(Span::styled(
        " VOL ",
        Style::default().fg(theme::FAINT).bg(theme::PANEL),
    ));
    spans.push(Span::styled(
        "▰".repeat(filled),
        Style::default().fg(theme::YELLOW_DIM).bg(theme::PANEL),
    ));
    spans.push(Span::styled(
        "▱".repeat(cells.saturating_sub(filled)),
        Style::default().fg(theme::FAINT).bg(theme::PANEL),
    ));
    if area.width >= 24 {
        spans.push(Span::styled(
            format!(" {:>3.0}%", effective * 100.0),
            Style::default().fg(theme::FAINT).bg(theme::PANEL),
        ));
    }
    primitives::right_line(frame, area, spans, Style::default().bg(theme::PANEL));
}

fn playback_source(app: &App) -> String {
    if app.current.is_none() {
        "NO OUTPUT".to_owned()
    } else if app.buffering {
        format!("{} BUFFERING", spinner(app.animation_tick))
    } else if app.wave.as_ref().is_some_and(|wave| wave.ready) {
        format!("{} MY WAVE", icons::WAVE)
    } else if app.mode == Mode::Demo {
        format!("{} SIMULATED", icons::CHECK)
    } else if app
        .current
        .as_ref()
        .is_some_and(|track| app.cached_ids.contains(&track.id))
    {
        format!("{} OFFLINE", icons::CHECK)
    } else {
        "STREAMING".to_owned()
    }
}

/// One-line contextual shortcut and status bar.
pub fn footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }
    primitives::paint(frame, area, theme::base());
    let hints = footer_hints(app, area.width);
    let right_text = footer_status(app, area.width);
    let right_width = (width(&right_text) + 1)
        .min(usize::from(area.width) / 2)
        .max(1) as u16;
    primitives::right_line(
        frame,
        Rect::new(
            area.x + area.width.saturating_sub(right_width + 2),
            area.y,
            right_width,
            1,
        ),
        vec![Span::styled(
            truncate(&right_text, usize::from(right_width)),
            Style::default().fg(footer_status_colour(app)).bg(theme::BG),
        )],
        theme::base(),
    );
    let left_width = area.width.saturating_sub(right_width + 5);
    draw_hints(frame, Rect::new(area.x + 2, area.y, left_width, 1), &hints);
}

fn footer_hints(app: &App, width: u16) -> Vec<(&'static str, &'static str)> {
    let wide = width >= 118;
    if let Some(overlay) = &app.overlay {
        return match overlay {
            Overlay::Search(search) => {
                if search.focus == crate::app::SearchFocus::Input {
                    vec![("tab", "results"), ("esc", "close")]
                } else {
                    vec![("esc", "input"), ("enter", "open")]
                }
            }
            Overlay::Help => vec![("esc", "close"), ("?", "close")],
            Overlay::TokenEntry(_) => vec![("enter", "connect"), ("esc", "cancel")],
        };
    }
    if !app.signed_in {
        let mut hints = vec![("d", "device sign-in"), ("t", "token")];
        if app.auth_busy {
            hints = vec![("esc", "cancel")];
        }
        hints.push(("q", "quit"));
        return hints;
    }
    if app.now_playing {
        if width < 96 {
            return vec![
                ("space", "pause"),
                ("n/p", "next/prev"),
                ("←/→", "seek"),
                ("v", "back"),
                ("?", "help"),
            ];
        }
        return vec![
            ("space", "play/pause"),
            ("n/p", "next/prev"),
            ("←/→", "seek"),
            ("j/k", "lyrics"),
            ("?", "help"),
            ("f", "like"),
            ("v/esc", "back"),
        ];
    }
    if app.focus == Focus::Main && app.section == Section::Wave {
        if app.wave.as_ref().is_some_and(|wave| wave.ready) {
            return vec![
                ("j/k", "move"),
                ("enter", "play"),
                ("n", "next"),
                ("w", "wave"),
                ("?", "help"),
                ("v", "player"),
            ];
        }
        return vec![
            ("enter", "tune in"),
            ("w", "wave"),
            ("/", "search"),
            ("?", "help"),
        ];
    }
    match app.focus {
        Focus::Sidebar => {
            let mut hints = vec![
                ("j/k", "browse"),
                ("enter", "open"),
                ("tab", "panes"),
                ("/", "search"),
                ("?", "help"),
            ];
            if wide {
                hints.push(("q", "quit"));
            }
            hints
        }
        Focus::Main if width < 96 => vec![
            ("j/k", "move"),
            ("enter", "play"),
            ("w", "wave"),
            ("/", "search"),
            ("?", "help"),
            ("v", "player"),
        ],
        Focus::Main => vec![
            ("j/k", "move"),
            ("enter", "play"),
            ("h/l", "panes"),
            ("/", "search"),
            ("w", "wave"),
            ("f", "like"),
            ("?", "help"),
            ("v", "player"),
        ],
        Focus::Inspector => vec![
            ("j/k", "move"),
            ("enter", "play"),
            ("i", "tab"),
            ("h", "back"),
            ("?", "help"),
        ],
    }
}

fn draw_hints(frame: &mut Frame<'_>, area: Rect, hints: &[(&'static str, &'static str)]) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    let limit = usize::from(area.width);
    for (index, (key, label)) in hints.iter().enumerate() {
        // The keycap contains one padding cell on each side, and the label span
        // itself starts with a space. Count the full rendered width so a hint
        // is either shown completely or omitted, never cut in the middle.
        let chunk = width(key) + 3 + width(label);
        let separator = if index == 0 { 0 } else { 2 };
        if used + chunk + separator > limit {
            break;
        }
        if index > 0 {
            spans.push(Span::styled("  ", Style::default().bg(theme::BG)));
            used += 2;
        }
        spans.push(keycap(key));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme::MUTED).bg(theme::BG),
        ));
        used += chunk;
    }
    primitives::line(frame, area, spans, theme::base());
}

fn footer_status(app: &App, width: u16) -> String {
    if let Some(notice) = app.visible_notice() {
        return format!("{}  {}", icons::DOT, notice.text);
    }
    if !app.signed_in {
        return format!("{}  NOT CONNECTED", icons::WARNING);
    }
    if let Some(retry) = app.retry_status() {
        return format!("{}  {retry}", spinner(app.animation_tick));
    }
    if app.library_loading {
        return format!("{}  SYNCING LIBRARY", spinner(app.animation_tick));
    }
    let suffix = if app.mode == Mode::Demo {
        "CACHED"
    } else {
        "OFFLINE"
    };
    if width < 96 {
        // Leave room for the shortcut bar on an 80-column terminal; the count
        // stays visible and the sidebar spells out what it means.
        format!("{}  {}", icons::CHECK, app.cached_count)
    } else if width < 118 {
        format!("{}  {} {suffix}", icons::CHECK, app.cached_count)
    } else {
        format!("{}  {} TRACKS {suffix}", icons::CHECK, app.cached_count)
    }
}

fn footer_status_colour(app: &App) -> Color {
    if let Some(notice) = app.visible_notice() {
        return theme::notice_colour(notice.kind);
    }
    if !app.signed_in {
        return theme::YELLOW_DIM;
    }
    if app.retry_status().is_some() || app.library_loading {
        theme::YELLOW_SOFT
    } else {
        theme::FAINT
    }
}

/// A reusable, compact key hint used by auth and overlay screens.
#[must_use]
pub fn keycap(key: &str) -> Span<'static> {
    Span::styled(
        format!(" {key} "),
        Style::default()
            .fg(theme::YELLOW_SOFT)
            .bg(theme::SURFACE)
            .add_modifier(Modifier::BOLD),
    )
}

/// A small label used above content headings.
pub fn eyebrow(frame: &mut Frame<'_>, area: Rect, label: &str) {
    primitives::line(
        frame,
        area,
        vec![
            Span::styled(
                "▰ ",
                Style::default()
                    .fg(theme::YELLOW_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                label.to_owned(),
                Style::default()
                    .fg(theme::FAINT)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        theme::base(),
    );
}

/// A soft wrapping message block, used by empty, loading, and error states.
pub fn message_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    icon: &str,
    title: &str,
    message: &str,
    colour: Color,
) {
    if area.height < 3 || area.width < 8 {
        return;
    }
    let shown = area.height.min(9);
    let panel_width = area.width.min(72);
    let box_area = Rect::new(
        area.x + (area.width - panel_width) / 2,
        area.y + (area.height - shown) / 2,
        panel_width,
        shown,
    );
    primitives::paint(
        frame,
        box_area,
        Style::default().bg(theme::PANEL).fg(theme::TEXT_SOFT),
    );
    primitives::line(
        frame,
        Rect::new(box_area.x + 2, box_area.y + 1, 3, 1),
        vec![Span::styled(
            icon,
            Style::default().fg(colour).bg(theme::PANEL),
        )],
        Style::default().bg(theme::PANEL),
    );
    primitives::line(
        frame,
        Rect::new(
            box_area.x + 5,
            box_area.y + 1,
            box_area.width.saturating_sub(7),
            1,
        ),
        vec![Span::styled(
            title.to_owned(),
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD),
        )],
        Style::default().bg(theme::PANEL),
    );
    let text_area = Rect::new(
        box_area.x + 5,
        box_area.y + 3,
        box_area.width.saturating_sub(7),
        box_area.height.saturating_sub(4),
    );
    frame.render_widget(
        Paragraph::new(message.to_owned())
            .style(Style::default().fg(theme::MUTED).bg(theme::PANEL))
            .wrap(ratatui::widgets::Wrap { trim: true }),
        text_area,
    );
    if box_area.height >= 6 {
        primitives::line(
            frame,
            Rect::new(
                box_area.x + 5,
                box_area.y + box_area.height - 2,
                box_area.width.saturating_sub(7),
                1,
            ),
            vec![
                keycap("r"),
                Span::styled(
                    " retry now   ",
                    Style::default().fg(theme::FAINT).bg(theme::PANEL),
                ),
                Span::styled(
                    "automatic retry is on",
                    Style::default().fg(theme::FAINT).bg(theme::PANEL),
                ),
            ],
            Style::default().bg(theme::PANEL),
        );
    }
}

/// A dim label used for the state of a collection or track.
#[must_use]
pub fn state_badge(label: &str, colour: Color) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default().fg(colour).bg(theme::SURFACE),
    )
}

/// Utility used by screenshot and help content to keep metadata aligned.
#[must_use]
pub fn detail_pair(label: &str, value: &str) -> String {
    format!("{}{}", fit(label, 10), value)
}

pub(crate) fn spinner(tick: u64) -> &'static str {
    icons::SPINNER[(tick as usize / 2) % icons::SPINNER.len()]
}
