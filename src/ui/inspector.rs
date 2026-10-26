//! The right-hand inspector: the play queue and synchronised lyrics.
//!
//! The inspector is intentionally narrow. It shows less, but it never becomes a
//! second content pane: one line of context, one obvious active item, and text
//! that wraps rather than clips.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus, InspectorTab};
use crate::ui::{components, icons, primitives, text, theme};

const TAB_QUEUE: &str = "Queue";
const TAB_LYRICS: &str = "Lyrics";

/// Draw the inspector in the space allocated by the shell.
pub fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width < 8 || area.height < 4 {
        return;
    }
    primitives::paint(frame, area, theme::panel());
    primitives::vertical_rule(
        frame,
        area.x,
        area.y,
        area.height,
        Style::default()
            .fg(if app.focus == Focus::Inspector {
                theme::YELLOW_DIM
            } else {
                theme::BORDER_SOFT
            })
            .bg(theme::PANEL),
    );

    let inner = Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(3),
        area.height,
    );
    draw_tabs(frame, app, Rect::new(inner.x, area.y + 1, inner.width, 1));
    primitives::rule(
        frame,
        Rect::new(inner.x, area.y + 2, inner.width, 1),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL),
    );

    let content = Rect::new(
        inner.x,
        area.y + 4,
        inner.width,
        area.height.saturating_sub(4),
    );
    if content.height == 0 {
        return;
    }
    match app.inspector_tab {
        InspectorTab::Queue => draw_queue(frame, app, content),
        InspectorTab::Lyrics => draw_lyrics(frame, app, content),
    }
}

fn draw_tabs(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width == 0 {
        return;
    }
    let compact = area.width < 22;
    let mut spans = Vec::new();
    for (index, tab) in [InspectorTab::Queue, InspectorTab::Lyrics]
        .into_iter()
        .enumerate()
    {
        let selected = app.inspector_tab == tab;
        if index > 0 {
            spans.push(Span::styled(" ", Style::default().bg(theme::PANEL)));
        }
        let icon = match tab {
            InspectorTab::Queue => icons::QUEUE,
            InspectorTab::Lyrics => icons::LYRICS,
        };
        let label = match tab {
            InspectorTab::Queue => TAB_QUEUE,
            InspectorTab::Lyrics => TAB_LYRICS,
        };
        let style = if selected {
            Style::default()
                .fg(theme::YELLOW)
                .bg(theme::SURFACE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::MUTED).bg(theme::PANEL)
        };
        spans.push(Span::styled(
            if compact {
                format!(" {icon} ")
            } else {
                format!(" {icon} {label} ")
            },
            style,
        ));
    }
    primitives::line(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        spans,
        Style::default().bg(theme::PANEL),
    );
    if area.width >= 30 {
        primitives::right_line(
            frame,
            Rect::new(area.x, area.y, area.width, 1),
            vec![Span::styled(
                "i",
                Style::default()
                    .fg(theme::FAINT)
                    .bg(theme::PANEL)
                    .add_modifier(Modifier::BOLD),
            )],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn draw_queue(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.queue.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                icons::QUEUE,
                Style::default().fg(theme::FAINT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Queue is empty",
                Style::default()
                    .fg(theme::TEXT_SOFT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press enter on a track",
                Style::default().fg(theme::MUTED),
            )),
            Line::from(Span::styled(
                "to start listening.",
                Style::default().fg(theme::MUTED),
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(theme::PANEL)),
            area,
        );
        return;
    }

    let visible = (usize::from(area.height) / 2).max(1);
    let start = text::window_start(app.queue_selected, app.queue.len(), visible);
    for (offset, track) in app.queue.iter().enumerate().skip(start).take(visible) {
        let index = offset;
        let y = area.y + (index - start) as u16 * 2;
        if y + 1 >= area.y + area.height {
            break;
        }
        let selected = index == app.queue_selected;
        let current = index == app.queue_cursor;
        let background = if selected {
            if app.focus == Focus::Inspector {
                theme::SELECT_FOCUS
            } else {
                theme::SELECT
            }
        } else if current {
            theme::PANEL_ALT
        } else {
            theme::PANEL
        };
        primitives::paint(
            frame,
            Rect::new(area.x, y, area.width, 2),
            Style::default().bg(background),
        );
        if selected {
            primitives::vertical_rule(
                frame,
                area.x.saturating_sub(1),
                y,
                2,
                Style::default()
                    .fg(if app.focus == Focus::Inspector {
                        theme::YELLOW
                    } else {
                        theme::YELLOW_DIM
                    })
                    .bg(theme::PANEL),
            );
        }
        let marker = if current {
            Span::styled(
                if app.playing {
                    format!("{} ", icons::PLAY)
                } else {
                    "▮ ".to_owned()
                },
                Style::default()
                    .fg(if app.playing {
                        theme::YELLOW
                    } else {
                        theme::YELLOW_DIM
                    })
                    .bg(background),
            )
        } else {
            Span::styled("  ", Style::default().bg(background))
        };
        let number = format!("{:>2} ", index + 1);
        let title_room = usize::from(area.width).saturating_sub(5);
        let title = text::truncate(&track.title, title_room);
        let title_style = Style::default()
            .fg(if current {
                theme::YELLOW_SOFT
            } else if selected {
                theme::TEXT
            } else {
                theme::TEXT_SOFT
            })
            .bg(background)
            .add_modifier(if current || selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                marker,
                Span::styled(number, Style::default().fg(theme::FAINT).bg(background)),
                Span::styled(title, title_style),
            ],
            Style::default().bg(background),
        );
        let artist_room = usize::from(area.width).saturating_sub(5);
        let artist = text::truncate(&track.artist, artist_room);
        primitives::line(
            frame,
            Rect::new(area.x + 2, y + 1, area.width.saturating_sub(2), 1),
            vec![
                Span::styled(artist, Style::default().fg(theme::MUTED).bg(background)),
                Span::styled(
                    format!("  {}", text::duration(track.duration())),
                    Style::default().fg(theme::FAINT).bg(background),
                ),
            ],
            Style::default().bg(background),
        );
    }
    if app.queue.len() > visible {
        primitives::right_line(
            frame,
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            vec![Span::styled(
                format!("{}/{}", app.queue_selected + 1, app.queue.len()),
                Style::default().fg(theme::FAINT).bg(theme::PANEL),
            )],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn draw_lyrics(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.current.is_none() {
        centered(
            frame,
            area,
            "Nothing playing",
            "Start a track to see its lyrics.",
            icons::LYRICS,
        );
        return;
    }
    if app.lyrics_loading {
        centered(
            frame,
            area,
            "Loading lyrics",
            "Looking for a time-coded version.",
            components::spinner(app.animation_tick),
        );
        return;
    }
    if let Some(error) = &app.lyrics_error {
        centered(frame, area, "Lyrics unavailable", error, icons::WARNING);
        return;
    }
    let Some(lyrics) = &app.lyrics else {
        centered(
            frame,
            area,
            "No lyrics",
            "This track may be instrumental.",
            icons::LYRICS,
        );
        return;
    };
    if lyrics.lines.is_empty() {
        centered(
            frame,
            area,
            "No lyrics",
            "This track may be instrumental.",
            icons::LYRICS,
        );
        return;
    }

    let width = usize::from(area.width).saturating_sub(2);
    if width < 4 {
        return;
    }
    let active = lyrics
        .active_index(app.position, app.duration)
        .unwrap_or(0)
        .min(lyrics.lines.len().saturating_sub(1));

    // Wrap first so the active line can be kept near the middle even when one
    // lyric spills across several rows.
    let mut rows: Vec<(usize, String)> = Vec::new();
    for (index, line) in lyrics.lines.iter().enumerate() {
        let wrapped = text::wrap(&line.text, width);
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
    // The sync badge owns the last row; reserve it before laying out text.
    let visible = area.height.saturating_sub(u16::from(area.height >= 10));
    let start = (target - i64::from(area.height / 3)).max(0) as usize;
    let start = start.min(rows.len().saturating_sub(usize::from(visible.max(1))));

    for (y, (line_index, (_, line))) in (area.y..).zip(
        rows.iter()
            .enumerate()
            .skip(start)
            .take(usize::from(visible)),
    ) {
        let is_active = line_index == active_visual;
        let distance = line_index.abs_diff(active_visual);
        let style = if is_active {
            Style::default()
                .fg(theme::YELLOW_SOFT)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD)
        } else if distance <= 2 {
            Style::default().fg(theme::TEXT_SOFT).bg(theme::PANEL)
        } else if distance <= 5 {
            Style::default().fg(theme::MUTED).bg(theme::PANEL)
        } else {
            Style::default().fg(theme::FAINT).bg(theme::PANEL)
        };
        let marker = if is_active { "▌" } else { " " };
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                Span::styled(
                    marker,
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

    // Footer of the lyric pane: source and a manual-scroll hint.
    if area.height >= 10 {
        let tag = if lyrics.synced {
            format!("{} SYNCED", icons::CHECK)
        } else {
            "PLAIN TEXT".to_owned()
        };
        primitives::right_line(
            frame,
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
            vec![Span::styled(
                if app.lyrics_offset == 0 {
                    tag
                } else {
                    format!("{tag}  +{}", app.lyrics_offset)
                },
                Style::default().fg(theme::FAINT).bg(theme::PANEL),
            )],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn centered(frame: &mut Frame<'_>, area: Rect, title: &str, message: &str, icon: &str) {
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
