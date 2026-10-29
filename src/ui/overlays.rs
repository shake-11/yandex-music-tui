//! Modal overlays: catalogue search, the keyboard map, and token entry.
//!
//! Overlays keep the shell visible around a lifted panel. They are fully
//! keyboard-driven and never steal focus from the application in a way the
//! footer cannot explain.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, Overlay, SearchFocus, SearchState, TokenEntry};
use crate::ui::{components, icons, primitives, text, theme};

/// Draw whichever overlay is active.
pub fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    match &app.overlay {
        Some(Overlay::Search(search)) => draw_search(frame, app, search, area),
        Some(Overlay::Help) => draw_help(frame, area),
        Some(Overlay::TokenEntry(entry)) => draw_token(frame, entry, area),
        None => {}
    }
}

struct Modal {
    area: Rect,
    inner: Rect,
}

fn modal(frame: &mut Frame<'_>, area: Rect, width: u16, height: u16) -> Modal {
    let width = width.min(area.width.saturating_sub(4)).max(20);
    let height = height.min(area.height.saturating_sub(2)).max(8);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let outer = Rect::new(x, y, width, height);

    // A hard-edged shadow gives the panel weight without a full-screen dimmer,
    // which would make the surrounding library flicker on every keypress.
    let shadow =
        Rect::new(x.saturating_add(2), y.saturating_add(1), width, height).intersection(area);
    primitives::paint(
        frame,
        shadow,
        Style::default().bg(theme::SHADOW).fg(theme::SHADOW),
    );
    frame.render_widget(Clear, outer);
    primitives::paint(
        frame,
        outer,
        Style::default().bg(theme::PANEL_ALT).fg(theme::TEXT),
    );
    frame.render_widget(
        ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER).bg(theme::PANEL_ALT)),
        outer,
    );
    let inner = Rect::new(
        outer.x + 2,
        outer.y + 1,
        outer.width.saturating_sub(4),
        outer.height.saturating_sub(2),
    );
    Modal { area: outer, inner }
}

fn panel_title(frame: &mut Frame<'_>, area: Rect, icon: &str, title: &str, right: &str) {
    if area.width < 4 || area.height == 0 {
        return;
    }
    primitives::line(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        vec![
            Span::styled(
                format!("{icon}  "),
                Style::default().fg(theme::YELLOW).bg(theme::PANEL_ALT),
            ),
            Span::styled(
                title.to_owned(),
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::PANEL_ALT)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        Style::default().bg(theme::PANEL_ALT),
    );
    primitives::right_line(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        vec![Span::styled(
            right.to_owned(),
            Style::default().fg(theme::FAINT).bg(theme::PANEL_ALT),
        )],
        Style::default().bg(theme::PANEL_ALT),
    );
}

fn draw_search(frame: &mut Frame<'_>, app: &App, search: &SearchState, area: Rect) {
    let width = if area.width >= 110 {
        100
    } else {
        area.width.saturating_sub(8)
    };
    let height = if area.height >= 34 {
        30
    } else {
        area.height.saturating_sub(4)
    };
    let modal = modal(frame, area, width, height);
    panel_title(frame, modal.inner, icons::SEARCH, "SEARCH", "CATALOGUE");
    let input_y = modal.inner.y + 2;
    let focused_input = search.focus == SearchFocus::Input;
    let caret = if focused_input {
        match app.animation_tick / 5 % 2 {
            0 => "█",
            _ => " ",
        }
    } else {
        ""
    };
    let prompt = if search.query.is_empty() {
        vec![
            Span::styled(
                "› ",
                Style::default()
                    .fg(if focused_input {
                        theme::YELLOW
                    } else {
                        theme::FAINT
                    })
                    .bg(theme::PANEL_ALT),
            ),
            Span::styled(
                "Search tracks, albums, artists, playlists",
                Style::default().fg(theme::FAINT).bg(theme::PANEL_ALT),
            ),
            Span::styled(caret, Style::default().fg(theme::YELLOW)),
        ]
    } else {
        vec![
            Span::styled(
                "› ",
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::PANEL_ALT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                text::truncate(
                    &search.query,
                    usize::from(modal.inner.width.saturating_sub(9)),
                ),
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::PANEL_ALT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                caret.to_owned(),
                Style::default().fg(theme::YELLOW).bg(theme::PANEL_ALT),
            ),
        ]
    };
    primitives::line(
        frame,
        Rect::new(modal.inner.x, input_y, modal.inner.width, 1),
        prompt,
        Style::default().bg(theme::PANEL_ALT),
    );
    if modal.inner.width >= 22 {
        primitives::right_line(
            frame,
            Rect::new(modal.inner.x, input_y, modal.inner.width, 1),
            vec![Span::styled(
                if focused_input {
                    "TAB · RESULTS"
                } else {
                    "ESC · INPUT"
                },
                Style::default().fg(theme::FAINT).bg(theme::PANEL_ALT),
            )],
            Style::default().bg(theme::PANEL_ALT),
        );
    }
    primitives::rule(
        frame,
        Rect::new(modal.inner.x, input_y + 1, modal.inner.width, 1),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL_ALT),
    );

    let results_area = Rect::new(
        modal.inner.x,
        input_y + 3,
        modal.inner.width,
        modal.inner.height.saturating_sub(8),
    );
    let status_y = modal.inner.y + modal.inner.height.saturating_sub(4);
    if search.query.trim().is_empty() {
        empty_search(frame, results_area);
    } else if search.loading {
        centered_lines(
            frame,
            results_area,
            &[
                (
                    components::spinner(app.animation_tick).to_string(),
                    Style::default().fg(theme::YELLOW),
                ),
                (
                    "Searching Yandex Music".into(),
                    Style::default().fg(theme::MUTED),
                ),
            ],
        );
    } else if let Some(error) = &search.error {
        centered_lines(
            frame,
            results_area,
            &[
                (icons::WARNING.to_owned(), Style::default().fg(theme::RED)),
                (error.clone(), Style::default().fg(theme::RED)),
            ],
        );
    } else if search.results.is_empty() {
        centered_lines(
            frame,
            results_area,
            &[
                (icons::SEARCH.to_owned(), Style::default().fg(theme::FAINT)),
                (
                    format!("No results for “{}”", search.query.trim()),
                    Style::default().fg(theme::MUTED),
                ),
            ],
        );
    } else {
        draw_results(frame, search, results_area);
    }

    primitives::rule(
        frame,
        Rect::new(
            modal.inner.x,
            status_y.saturating_sub(1),
            modal.inner.width,
            1,
        ),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL_ALT),
    );
    let status = if search.loading {
        format!("{}  SEARCHING", components::spinner(app.animation_tick))
    } else if search.error.is_some() {
        format!("{}  SEARCH FAILED", icons::WARNING)
    } else if search.results.is_empty() {
        "↑↓ move · enter open · esc close".to_owned()
    } else {
        format!(
            "{} RESULTS  ·  ↑↓ MOVE  ·  ENTER OPEN  ·  ESC CLOSE",
            search.result_count()
        )
    };
    primitives::line(
        frame,
        Rect::new(modal.inner.x, status_y, modal.inner.width, 1),
        vec![Span::styled(
            status,
            Style::default()
                .fg(if search.error.is_some() {
                    theme::RED
                } else {
                    theme::FAINT
                })
                .bg(theme::PANEL_ALT),
        )],
        Style::default().bg(theme::PANEL_ALT),
    );
    if !search.results.is_empty() {
        primitives::right_line(
            frame,
            Rect::new(modal.inner.x, status_y, modal.inner.width, 1),
            vec![Span::styled(
                format!("{:02}/{:02}", search.selected + 1, search.result_count()),
                Style::default().fg(theme::YELLOW_DIM).bg(theme::PANEL_ALT),
            )],
            Style::default().bg(theme::PANEL_ALT),
        );
    }
    let _ = &modal.area;
}

fn empty_search(frame: &mut Frame<'_>, area: Rect) {
    if area.height < 5 {
        return;
    }
    let lines = vec![
        Line::from(Span::styled(
            "TYPE TO SEARCH THE CATALOGUE",
            Style::default()
                .fg(theme::FAINT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Tracks, albums, artists, and playlists appear together.",
            Style::default().fg(theme::MUTED),
        )),
        Line::from(Span::styled(
            "Press Tab to move into the results.",
            Style::default().fg(theme::MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(theme::PANEL_ALT)),
        area,
    );
}

enum Row {
    Header(&'static str),
    Track(usize, usize),
    Album(usize, usize),
    Artist(usize, usize),
    Playlist(usize, usize),
}

impl Row {
    fn hit(&self) -> Option<usize> {
        match self {
            Self::Header(_) => None,
            Self::Track(hit, _)
            | Self::Album(hit, _)
            | Self::Artist(hit, _)
            | Self::Playlist(hit, _) => Some(*hit),
        }
    }
}

fn draw_results(frame: &mut Frame<'_>, search: &SearchState, area: Rect) {
    if area.height == 0 || area.width < 16 {
        return;
    }
    let mut rows = Vec::new();
    let mut hit = 0usize;
    if !search.results.tracks.is_empty() {
        rows.push(Row::Header("TRACKS"));
        for index in 0..search.results.tracks.len() {
            rows.push(Row::Track(hit, index));
            hit += 1;
        }
    }
    if !search.results.albums.is_empty() {
        rows.push(Row::Header("ALBUMS"));
        for index in 0..search.results.albums.len() {
            rows.push(Row::Album(hit, index));
            hit += 1;
        }
    }
    if !search.results.artists.is_empty() {
        rows.push(Row::Header("ARTISTS"));
        for index in 0..search.results.artists.len() {
            rows.push(Row::Artist(hit, index));
            hit += 1;
        }
    }
    if !search.results.playlists.is_empty() {
        rows.push(Row::Header("PLAYLISTS"));
        for index in 0..search.results.playlists.len() {
            rows.push(Row::Playlist(hit, index));
            hit += 1;
        }
    }
    let selected_row = rows
        .iter()
        .position(|row| row.hit() == Some(search.selected))
        .unwrap_or(0);
    let visible = usize::from(area.height);
    let start = text::window_start(selected_row, rows.len(), visible);
    let focused = search.focus == SearchFocus::Results;

    for (offset, row) in rows.iter().skip(start).take(visible).enumerate() {
        let absolute = start + offset;
        let y = area.y + offset as u16;
        if let Row::Header(label) = row {
            primitives::line(
                frame,
                Rect::new(area.x, y, area.width, 1),
                vec![
                    Span::styled(
                        format!("{label}  "),
                        Style::default()
                            .fg(theme::FAINT)
                            .bg(theme::PANEL_ALT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "─".repeat(usize::from(area.width).saturating_sub(label.len() + 2)),
                        Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL_ALT),
                    ),
                ],
                Style::default().bg(theme::PANEL_ALT),
            );
            continue;
        }
        let is_selected = row.hit() == Some(search.selected);
        let background = if is_selected {
            if focused {
                theme::SELECT_FOCUS
            } else {
                theme::SELECT
            }
        } else {
            theme::PANEL_ALT
        };
        primitives::paint(
            frame,
            Rect::new(area.x, y, area.width, 1),
            Style::default().bg(background),
        );
        let marker = Span::styled(
            if is_selected { "▌ " } else { "  " },
            Style::default()
                .fg(if focused {
                    theme::YELLOW
                } else {
                    theme::YELLOW_DIM
                })
                .bg(background),
        );
        let duration = match row {
            Row::Track(_, index) => search
                .results
                .tracks
                .get(*index)
                .map(|track| text::duration(track.duration()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        let right_width = if duration.is_empty() {
            0
        } else {
            text::width(&duration) + 2
        };
        let text_width = usize::from(area.width).saturating_sub(4 + right_width);
        let (icon, title, meta) = match row {
            Row::Track(_, index) => {
                let track = &search.results.tracks[*index];
                (
                    "♪",
                    track.title.clone(),
                    if track.artist.is_empty() {
                        track.album.clone().unwrap_or_default()
                    } else {
                        track.artist.clone()
                    },
                )
            }
            Row::Album(_, index) => {
                let album = &search.results.albums[*index];
                (
                    icons::ALBUM,
                    album.title.clone(),
                    format!("{} · ALBUM", album.artist),
                )
            }
            Row::Artist(_, index) => {
                let artist = &search.results.artists[*index];
                (icons::ARTIST, artist.name.clone(), "ARTIST".to_owned())
            }
            Row::Playlist(_, index) => {
                let playlist = &search.results.playlists[*index];
                (
                    icons::PLAYLIST,
                    playlist.title.clone(),
                    format!("{} · PLAYLIST", playlist.owner),
                )
            }
            Row::Header(_) => continue,
        };
        let title_room = text_width.saturating_sub(2);
        let title_text = text::truncate(&title, title_room);
        let used = text::width(&title_text);
        let meta_room = text_width.saturating_sub(used + 3);
        let meta_text = if meta_room >= 4 {
            format!("  {}", text::truncate(&meta, meta_room - 2))
        } else {
            String::new()
        };
        let mut spans = vec![
            marker,
            Span::styled(
                format!("{icon}  "),
                Style::default()
                    .fg(if is_selected {
                        theme::YELLOW
                    } else {
                        theme::FAINT
                    })
                    .bg(background),
            ),
            Span::styled(
                title_text,
                Style::default()
                    .fg(if is_selected {
                        theme::TEXT
                    } else {
                        theme::TEXT_SOFT
                    })
                    .bg(background)
                    .add_modifier(if is_selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(meta_text, Style::default().fg(theme::MUTED).bg(background)),
        ];
        let pad =
            usize::from(area.width).saturating_sub(primitives::spans_width(&spans) + right_width);
        spans.push(Span::styled(
            " ".repeat(pad),
            Style::default().bg(background),
        ));
        if !duration.is_empty() {
            spans.push(Span::styled(
                duration,
                Style::default().fg(theme::FAINT).bg(background),
            ));
            spans.push(Span::styled("  ", Style::default().bg(background)));
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            spans,
            Style::default().bg(background),
        );
        let _ = absolute;
    }
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let width = if area.width >= 130 {
        112
    } else {
        area.width.saturating_sub(8)
    };
    let height = if area.height >= 34 {
        30
    } else {
        area.height.saturating_sub(4)
    };
    let modal = modal(frame, area, width, height);
    panel_title(
        frame,
        modal.inner,
        icons::HEADPHONES,
        "HELP / KEYBOARD",
        "ESC TO CLOSE",
    );
    if modal.inner.height < 6 {
        return;
    }
    let body = Rect::new(
        modal.inner.x,
        modal.inner.y + 2,
        modal.inner.width,
        modal.inner.height.saturating_sub(3),
    );
    let sections: [(&str, &[(&str, &str)]); 4] = [
        (
            "NAVIGATION",
            &[
                ("j / k", "Move down / up"),
                ("g / G", "First / last"),
                ("h / l", "Previous / next pane"),
                ("tab", "Cycle panes"),
                ("enter", "Play or open"),
                ("esc", "Back"),
            ],
        ),
        (
            "LIBRARY",
            &[
                ("/", "Search the catalogue"),
                ("1 – 5", "Jump to a saved section"),
                ("w", "Start or show My Wave"),
                ("f", "Like / unlike"),
                ("d", "Keep offline"),
                ("r", "Retry now"),
            ],
        ),
        (
            "PLAYBACK",
            &[
                ("space", "Play / pause"),
                ("n / p", "Next / previous"),
                ("← / →", "Seek in Now Playing"),
                ("j / k", "Scroll lyrics"),
                ("m", "Mute"),
                ("+ / -", "Volume"),
            ],
        ),
        (
            "INTERFACE",
            &[
                ("v", "Open Now Playing"),
                ("i", "Switch inspector tab"),
                ("[ / ]", "Queue / lyrics"),
                ("?", "This keyboard map"),
                ("q", "Quit"),
                ("ctrl+c", "Quit from anywhere"),
            ],
        ),
    ];

    // Two columns remain readable on an 80×24 terminal and keep all four key
    // groups visible without a scroll offset.
    let columns = if body.width >= 64 { 2 } else { 1 };
    if columns == 2 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body);
        let per_column = sections.len().div_ceil(2);
        for (column, area) in split.iter().enumerate() {
            let start = column * per_column;
            let end = (start + per_column).min(sections.len());
            let mut y = area.y;
            for (title, keys) in &sections[start..end] {
                y = draw_help_section(frame, *area, y, title, keys);
            }
        }
    } else {
        let mut y = body.y;
        for (title, keys) in &sections {
            y = draw_help_section(frame, body, y, title, keys);
            if y >= body.y + body.height {
                break;
            }
        }
    }
    if modal.inner.height >= 8 {
        primitives::right_line(
            frame,
            Rect::new(
                modal.inner.x,
                modal.inner.y + modal.inner.height - 1,
                modal.inner.width,
                1,
            ),
            vec![Span::styled(
                "UNOFFICIAL CLIENT · NOT AFFILIATED WITH YANDEX",
                Style::default().fg(theme::FAINT).bg(theme::PANEL_ALT),
            )],
            Style::default().bg(theme::PANEL_ALT),
        );
    }
}

fn draw_help_section(
    frame: &mut Frame<'_>,
    area: Rect,
    mut y: u16,
    title: &str,
    keys: &[(&str, &str)],
) -> u16 {
    if y >= area.y + area.height {
        return y;
    }
    primitives::line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![Span::styled(
            format!("▰ {title}"),
            Style::default()
                .fg(theme::YELLOW_DIM)
                .bg(theme::PANEL_ALT)
                .add_modifier(Modifier::BOLD),
        )],
        Style::default().bg(theme::PANEL_ALT),
    );
    y += 1;
    for (key, description) in keys {
        if y >= area.y + area.height {
            break;
        }
        let mut spans = vec![
            Span::styled(
                format!(" {key} "),
                Style::default()
                    .fg(theme::YELLOW_SOFT)
                    .bg(theme::SURFACE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {description}"),
                Style::default().fg(theme::TEXT_SOFT).bg(theme::PANEL_ALT),
            ),
        ];
        let used = primitives::spans_width(&spans);
        if used < usize::from(area.width) {
            spans.push(Span::styled(
                " ".repeat(usize::from(area.width) - used),
                Style::default().bg(theme::PANEL_ALT),
            ));
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            spans,
            Style::default().bg(theme::PANEL_ALT),
        );
        y += 1;
    }
    y + 1
}

fn draw_token(frame: &mut Frame<'_>, entry: &TokenEntry, area: Rect) {
    let width = if area.width >= 90 {
        74
    } else {
        area.width.saturating_sub(8)
    };
    let height = if area.height >= 24 {
        14
    } else {
        area.height.saturating_sub(4).max(10)
    };
    let modal = modal(frame, area, width, height);
    panel_title(
        frame,
        modal.inner,
        icons::ARTIST,
        "CONNECT WITH A TOKEN",
        "ESC TO CANCEL",
    );
    let mut y = modal.inner.y + 2;
    let explanation = "Paste a Yandex Music access token. It is stored only in your config with owner-only permissions.";
    for line in text::wrap(explanation, usize::from(modal.inner.width)) {
        if y >= modal.inner.y + modal.inner.height {
            break;
        }
        primitives::line(
            frame,
            Rect::new(modal.inner.x, y, modal.inner.width, 1),
            vec![Span::styled(
                line,
                Style::default().fg(theme::MUTED).bg(theme::PANEL_ALT),
            )],
            Style::default().bg(theme::PANEL_ALT),
        );
        y += 1;
    }
    y += 1;
    let field = Rect::new(modal.inner.x, y, modal.inner.width, 1);
    primitives::paint(
        frame,
        field,
        Style::default().bg(theme::SURFACE).fg(theme::TEXT),
    );
    let masked = if entry.value.is_empty() {
        if entry.busy {
            format!("{}  Checking the token…", components::spinner(0))
        } else {
            "Paste a token, then press Enter".to_owned()
        }
    } else if entry.busy {
        format!("{}  Checking the token…", components::spinner(0))
    } else {
        mask_token(&entry.value)
    };
    primitives::line(
        frame,
        field,
        vec![
            Span::styled(
                if entry.busy { "  " } else { " █ " },
                Style::default().fg(theme::YELLOW).bg(theme::SURFACE),
            ),
            Span::styled(
                masked,
                Style::default()
                    .fg(if entry.value.is_empty() {
                        theme::FAINT
                    } else {
                        theme::TEXT
                    })
                    .bg(theme::SURFACE),
            ),
        ],
        Style::default().bg(theme::SURFACE),
    );
    y += 1;
    if let Some(error) = &entry.error {
        if y < modal.inner.y + modal.inner.height {
            primitives::line(
                frame,
                Rect::new(modal.inner.x, y, modal.inner.width, 1),
                vec![
                    Span::styled(
                        format!("{}  ", icons::WARNING),
                        Style::default().fg(theme::RED),
                    ),
                    Span::styled(
                        text::truncate(error, usize::from(modal.inner.width.saturating_sub(3))),
                        Style::default().fg(theme::RED).bg(theme::PANEL_ALT),
                    ),
                ],
                Style::default().bg(theme::PANEL_ALT),
            );
        }
    }
    let hint_y = modal.inner.y + modal.inner.height.saturating_sub(3);
    if hint_y < area.y + area.height && modal.inner.height >= 6 {
        primitives::rule(
            frame,
            Rect::new(
                modal.inner.x,
                hint_y.saturating_sub(1),
                modal.inner.width,
                1,
            ),
            Style::default().fg(theme::BORDER_SOFT).bg(theme::PANEL_ALT),
        );
        primitives::line(
            frame,
            Rect::new(modal.inner.x, hint_y, modal.inner.width, 1),
            vec![
                components::keycap("enter"),
                Span::styled(" connect   ", Style::default().fg(theme::MUTED)),
                components::keycap("esc"),
                Span::styled(" cancel   ", Style::default().fg(theme::MUTED)),
                Span::styled(
                    "tokens are never echoed in full",
                    Style::default().fg(theme::FAINT),
                ),
            ],
            Style::default().bg(theme::PANEL_ALT),
        );
    }
}

fn mask_token(value: &str) -> String {
    let visible = 4usize;
    let count = value.chars().count();
    if count <= visible {
        return "•".repeat(count);
    }
    let tail: String = value
        .chars()
        .rev()
        .take(visible)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}{}", "•".repeat((count - visible).min(48)), tail)
}

fn centered_lines(frame: &mut Frame<'_>, area: Rect, lines: &[(String, Style)]) {
    if area.height == 0 {
        return;
    }
    let lines: Vec<Line> = lines
        .iter()
        .map(|(text, style)| Line::from(Span::styled(text.clone(), *style)))
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(theme::PANEL_ALT)),
        area,
    );
}
