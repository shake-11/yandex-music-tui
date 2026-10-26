//! First-run authentication: a quiet, composed landing screen.
//!
//! The screen offers the two routes the rest of the runtime supports: OAuth
//! device authorisation and a pasted access token. Status text always reflects
//! the automatic connection retries happening in the background.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::{components, cover, icons, primitives, text, theme};

/// Draw the signed-out screen.
pub fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    primitives::paint(frame, area, theme::base());
    if area.width < 40 || area.height < 12 {
        draw_compact(frame, app, area);
        return;
    }

    let split = area.width >= 100 && area.height >= 24;
    let content_area = if split {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
            .split(area);
        draw_atmosphere(frame, columns[0], app);
        columns[1]
    } else {
        area
    };

    if split {
        primitives::vertical_rule(
            frame,
            content_area.x,
            area.y,
            area.height,
            Style::default().fg(theme::BORDER_SOFT).bg(theme::BG),
        );
    }

    let panel_width = if split {
        content_area.width.saturating_sub(10).min(62)
    } else {
        area.width.saturating_sub(8).min(68)
    }
    .max(30);
    let x = content_area.x + (content_area.width.saturating_sub(panel_width)) / 2;
    let content_height = if app.auth_busy { 17 } else { 18 };
    let y = area.y + area.height.saturating_sub(content_height + 2) / 2;
    let content = Rect::new(x, y, panel_width, content_height);

    brand(frame, Rect::new(content.x, area.y + 1, content.width, 1));
    draw_auth_body(frame, app, content);
    draw_footer_hints(frame, app, area);
}

fn draw_atmosphere(frame: &mut Frame<'_>, area: Rect, app: &App) {
    primitives::paint(frame, area, theme::base());
    let art = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(2));
    cover::draw_atmosphere(frame, art, &format!("auth / {} / night", app.profile.login));
    // A small, high-contrast signature sits on the image.
    let bar = Rect::new(area.x + 3, area.y + 3, area.width.saturating_sub(6), 1);
    primitives::line(
        frame,
        bar,
        vec![
            Span::styled(
                "▰ ",
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::SHADOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "YANDEX MUSIC",
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::SHADOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " / TUI",
                Style::default().fg(theme::MUTED).bg(theme::SHADOW),
            ),
        ],
        Style::default().bg(theme::SHADOW),
    );
    let tagline = Rect::new(
        area.x + 3,
        area.y + area.height.saturating_sub(4),
        area.width.saturating_sub(6),
        1,
    );
    primitives::line(
        frame,
        tagline,
        vec![Span::styled(
            "A QUIET PLACE FOR LOUD FEELINGS",
            Style::default().fg(theme::TEXT_SOFT).bg(theme::SHADOW),
        )],
        Style::default().bg(theme::SHADOW),
    );
    primitives::right_line(
        frame,
        Rect::new(
            area.x + 3,
            area.y + area.height.saturating_sub(2),
            area.width.saturating_sub(6),
            1,
        ),
        vec![Span::styled(
            "UNOFFICIAL · NOT AFFILIATED WITH YANDEX",
            Style::default().fg(theme::MUTED).bg(theme::BG),
        )],
        theme::base(),
    );
}

fn brand(frame: &mut Frame<'_>, area: Rect) {
    if area.width < 10 {
        return;
    }
    primitives::line(
        frame,
        area,
        vec![
            Span::styled(
                "▰ ",
                Style::default()
                    .fg(theme::YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "YANDEX",
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" / MUSIC", Style::default().fg(theme::MUTED)),
            Span::styled(
                " TUI ",
                Style::default()
                    .fg(theme::YELLOW_SOFT)
                    .bg(theme::SURFACE)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        theme::base(),
    );
}

fn draw_auth_body(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut y = area.y;
    components::eyebrow(
        frame,
        Rect::new(area.x, y, area.width, 1),
        "SIGN IN · NO BROWSER EXTENSION REQUIRED",
    );
    y += 2;
    primitives::line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![Span::styled(
            "Your library,",
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::BG)
                .add_modifier(Modifier::BOLD),
        )],
        theme::base(),
    );
    y += 1;
    primitives::line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![
            Span::styled(
                "without the ",
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "noise.",
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        theme::base(),
    );
    y += 2;
    let blurb = text::wrap(
        "A keyboard-first player for the music you already love. Sign in once and keep your library, playlists, and lyrics within reach.",
        usize::from(area.width),
    );
    for line in blurb.into_iter().take(3) {
        if y >= area.y + area.height {
            return;
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![Span::styled(
                line,
                Style::default().fg(theme::MUTED).bg(theme::BG),
            )],
            theme::base(),
        );
        y += 1;
    }
    y += 1;

    if let Some(prompt) = &app.auth_device {
        draw_device_prompt(frame, area, y, app, prompt);
        return;
    }
    if app.auth_busy {
        draw_waiting(frame, area, y, app);
        return;
    }

    draw_options(frame, area, y, app);
}

fn draw_options(frame: &mut Frame<'_>, area: Rect, y: u16, app: &App) {
    let options = [
        (
            "d",
            "Sign in on this device",
            "Open a verification page, enter a short code, done.",
        ),
        (
            "t",
            "Paste an access token",
            "Useful when a token already lives in your password store.",
        ),
    ];
    let mut y = y;
    for (key, title, description) in options {
        if y + 2 >= area.y + area.height {
            break;
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                components::keycap(key),
                Span::styled(
                    format!("  {title}"),
                    Style::default()
                        .fg(theme::TEXT_SOFT)
                        .bg(theme::BG)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            theme::base(),
        );
        y += 1;
        let description_width = usize::from(area.width.saturating_sub(4));
        for line in text::wrap(description, description_width)
            .into_iter()
            .take(2)
        {
            if y >= area.y + area.height {
                break;
            }
            primitives::line(
                frame,
                Rect::new(area.x + 4, y, area.width.saturating_sub(4), 1),
                vec![Span::styled(
                    line,
                    Style::default().fg(theme::MUTED).bg(theme::BG),
                )],
                theme::base(),
            );
            y += 1;
        }
        y += 1;
    }

    if let Some(error) = &app.auth_error {
        draw_error(frame, area, y, error, app);
        return;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                components::keycap("q"),
                Span::styled(
                    "  Leave, quietly",
                    Style::default().fg(theme::MUTED).bg(theme::BG),
                ),
            ],
            theme::base(),
        );
    }
}

fn draw_waiting(frame: &mut Frame<'_>, area: Rect, y: u16, app: &App) {
    let spinner = components::spinner(app.animation_tick);
    primitives::line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![
            Span::styled(format!("{spinner}  "), Style::default().fg(theme::YELLOW)),
            Span::styled(
                text::truncate(&app.auth_note, usize::from(area.width.saturating_sub(3))),
                Style::default()
                    .fg(theme::TEXT_SOFT)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            ),
        ],
        theme::base(),
    );
    if let Some(error) = &app.auth_error {
        draw_error(frame, area, y + 2, error, app);
    } else if y + 2 < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y + 2, area.width, 1),
            vec![
                Span::styled(
                    "Connection failures retry automatically. ",
                    Style::default().fg(theme::MUTED).bg(theme::BG),
                ),
                components::keycap("esc"),
                Span::styled(" cancel", Style::default().fg(theme::MUTED).bg(theme::BG)),
            ],
            theme::base(),
        );
    }
}

fn draw_device_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    y: u16,
    app: &App,
    prompt: &crate::app::DevicePrompt,
) {
    let mut y = y;
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                Span::styled("YOUR CODE  ", Style::default().fg(theme::FAINT)),
                Span::styled(
                    format!(" {} ", prompt.user_code),
                    Style::default()
                        .fg(theme::BG)
                        .bg(theme::YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            theme::base(),
        );
        y += 2;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![Span::styled(
                "Open this address in any browser:",
                Style::default().fg(theme::MUTED).bg(theme::BG),
            )],
            theme::base(),
        );
        y += 1;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![Span::styled(
                text::truncate(&prompt.verification_url, usize::from(area.width)),
                Style::default()
                    .fg(theme::YELLOW_SOFT)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            )],
            theme::base(),
        );
        y += 2;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                Span::styled(
                    format!("{}  ", components::spinner(app.animation_tick)),
                    Style::default().fg(theme::YELLOW),
                ),
                Span::styled(
                    "Waiting for confirmation",
                    Style::default().fg(theme::TEXT_SOFT),
                ),
            ],
            theme::base(),
        );
        y += 1;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                Span::styled(
                    "This screen updates by itself. ",
                    Style::default().fg(theme::MUTED),
                ),
                components::keycap("esc"),
                Span::styled(" cancel", Style::default().fg(theme::MUTED)),
            ],
            theme::base(),
        );
    }
}

fn draw_error(frame: &mut Frame<'_>, area: Rect, y: u16, error: &str, app: &App) {
    if y >= area.y + area.height {
        return;
    }
    let wrapped = text::wrap(error, usize::from(area.width.saturating_sub(2)));
    let mut y = y;
    for (index, line) in wrapped.into_iter().take(3).enumerate() {
        if y >= area.y + area.height {
            break;
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            vec![
                Span::styled(
                    if index == 0 {
                        format!("{}  ", icons::WARNING)
                    } else {
                        "   ".to_owned()
                    },
                    Style::default().fg(theme::RED),
                ),
                Span::styled(line, Style::default().fg(theme::RED).bg(theme::BG)),
            ],
            theme::base(),
        );
        y += 1;
    }
    if y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(area.x, y + 1, area.width, 1),
            vec![
                components::keycap("d"),
                Span::styled(" try again   ", Style::default().fg(theme::MUTED)),
                components::keycap("t"),
                Span::styled(" use a token", Style::default().fg(theme::MUTED)),
            ],
            theme::base(),
        );
    }
    let _ = app;
}

fn draw_footer_hints(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.height < 3 {
        return;
    }
    primitives::rule(
        frame,
        Rect::new(
            area.x + 3,
            area.y + area.height - 3,
            area.width.saturating_sub(6),
            1,
        ),
        Style::default().fg(theme::BORDER_SOFT).bg(theme::BG),
    );
    let mut hints = Vec::new();
    if app.auth_busy {
        hints.push(("esc", "cancel sign-in"));
    } else {
        hints.push(("d", "device sign-in"));
        hints.push(("t", "access token"));
    }
    hints.push(("q", "quit"));
    // At 80 columns the version tag and the full hint row would collide.
    // Drop the final hint rather than letting it disappear under the version.
    if area.width >= 96 {
        hints.push(("?", "help"));
    }
    let mut spans = Vec::new();
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("   ", Style::default().bg(theme::BG)));
        }
        spans.push(components::keycap(key));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme::MUTED).bg(theme::BG),
        ));
    }
    primitives::line(
        frame,
        Rect::new(
            area.x + 3,
            area.y + area.height - 2,
            area.width.saturating_sub(6),
            1,
        ),
        spans,
        theme::base(),
    );
    primitives::right_line(
        frame,
        Rect::new(
            area.x + 3,
            area.y + area.height - 2,
            area.width.saturating_sub(6),
            1,
        ),
        vec![Span::styled(
            format!("v{} · UNOFFICIAL", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme::FAINT).bg(theme::BG),
        )],
        theme::base(),
    );
}

fn draw_compact(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            "YANDEX MUSIC TUI",
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            if app.auth_busy {
                app.auth_note.clone()
            } else {
                "d · device sign-in".into()
            },
            Style::default().fg(theme::TEXT_SOFT),
        )),
        Line::from(Span::styled(
            if app.auth_busy {
                "esc · cancel"
            } else {
                "t · paste token"
            },
            Style::default().fg(theme::MUTED),
        )),
    ];
    if let Some(prompt) = &app.auth_device {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            prompt.user_code.clone(),
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            prompt.verification_url.clone(),
            Style::default().fg(theme::MUTED),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(theme::base()),
        area,
    );
}
