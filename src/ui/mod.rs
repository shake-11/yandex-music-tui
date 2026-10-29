//! Rendering for the terminal interface.
//!
//! The renderer is a pure function of [`App`]. It never blocks, never touches
//! the network, and never assumes that a pane is large enough: every component
//! clips itself rather than panicking on tiny terminals.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Overlay};

pub mod auth;
pub mod components;
pub mod content;
pub mod cover;
pub mod icons;
pub mod inspector;
pub mod now_playing;
pub mod overlays;
pub mod primitives;
pub mod text;
pub mod theme;

/// Draw one complete frame.
pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    primitives::paint(frame, area, theme::base());
    if area.width < 48 || area.height < 12 {
        draw_too_small(frame, area);
        return;
    }

    if !app.signed_in {
        auth::draw(frame, app, area);
    } else {
        let header_height = if area.height >= 16 { 2 } else { 1 };
        let footer_height = 1;
        let player_height = if area.height >= 20 { 3 } else { 2 };
        let body_height = area
            .height
            .saturating_sub(header_height + footer_height + player_height);
        let header = Rect::new(area.x, area.y, area.width, header_height);
        let body = Rect::new(area.x, area.y + header_height, area.width, body_height);
        let player = Rect::new(
            area.x,
            area.y + header_height + body_height,
            area.width,
            player_height,
        );
        let footer = Rect::new(
            area.x,
            area.y + area.height - footer_height,
            area.width,
            footer_height,
        );

        components::header(frame, app, header);
        if app.now_playing {
            now_playing::draw(frame, app, body);
        } else {
            draw_shell(frame, app, body);
        }
        components::player_bar(frame, app, player);
        components::footer(frame, app, footer);
    }

    if matches!(
        app.overlay,
        Some(Overlay::Search(_) | Overlay::Help | Overlay::TokenEntry(_))
    ) {
        overlays::draw(frame, app, area);
    }
}

fn draw_shell(frame: &mut Frame<'_>, app: &App, body: Rect) {
    if body.width == 0 || body.height == 0 {
        return;
    }
    let wide = body.width >= app.compact_sidebar_below;
    let funnel = body.width < 72;
    let (sidebar_width, inspector_width, compact) = if wide {
        let sidebar = (body.width * 30 / 100).max(22).min(body.width / 2);
        let inspector = (body.width * 20 / 100)
            .max(18)
            .min(body.width.saturating_sub(sidebar + 24));
        (sidebar, inspector, false)
    } else {
        let sidebar = body.width.min(7);
        if funnel {
            (sidebar, 0, true)
        } else {
            let inspector = (body.width * 32 / 100)
                .clamp(20, 34)
                .min(body.width.saturating_sub(sidebar + 28));
            (sidebar, inspector, true)
        }
    };
    let content_width = body.width.saturating_sub(sidebar_width + inspector_width);
    let sidebar = Rect::new(body.x, body.y, sidebar_width, body.height);
    let content = Rect::new(body.x + sidebar_width, body.y, content_width, body.height);
    let inspector = Rect::new(
        body.x + sidebar_width + content_width,
        body.y,
        inspector_width,
        body.height,
    );

    components::sidebar(frame, app, sidebar, compact);
    content::draw(frame, app, content);
    if inspector_width > 0 {
        inspector::draw(frame, app, inspector);
    }
}

fn draw_too_small(frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(
            "▰ YANDEX MUSIC TUI",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The terminal is a little too small.",
            Style::default().fg(theme::TEXT_SOFT),
        )),
        Line::from(Span::styled(
            format!("{} × {} · try at least 48 × 12", area.width, area.height),
            Style::default().fg(theme::MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Resize the window, or press q to leave.",
            Style::default().fg(theme::FAINT),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(ratatui::layout::Alignment::Center)
            .style(theme::base()),
        area,
    );
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crossterm::event::{KeyCode, KeyEvent};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    use super::*;
    use crate::api::demo;
    use crate::app::{
        Action, App, Focus, InspectorTab, Mode, SearchFocus, SearchState, Section, View,
    };
    use crate::config::Config;

    fn render(app: &App, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| draw(frame, app))
            .expect("the interface must render without a terminal");
        terminal.backend().buffer().clone()
    }

    fn line(buffer: &Buffer, width: u16, y: u16) -> String {
        (0..width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    }

    fn whole(buffer: &Buffer, width: u16, height: u16) -> String {
        (0..height)
            .map(|y| line(buffer, width, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn shell_renders_at_wide_and_compact_sizes() {
        let config = Config::default();
        let mut app = App::demo(&config);

        let wide = whole(&render(&app, 200, 60), 200, 60);
        assert!(wide.contains("YOUR LIBRARY"));
        assert!(wide.contains("RECENTLY LIKED"));
        assert!(wide.contains("My Wave"));
        assert!(wide.contains("Albums"));
        assert!(wide.contains("Search"));
        assert!(wide.contains("Queue"));
        assert!(wide.contains("3 TRACKS CACHED"));

        app.section = Section::Home;
        app.view = View::Library(Section::Home);
        app.focus = Focus::Main;
        let compact = render(&app, 80, 24);
        let text = whole(&compact, 80, 24);
        assert!(text.contains("Good evening, Alex"));
        assert!(text.contains("RECENTLY LIKED"));
        assert!(text.contains("SIMULATED"));

        // The footer must stop between hints, never in the middle of one.
        let footer = line(&compact, 80, 23);
        for hint in [
            "j/k  move",
            "enter  play",
            "w  wave",
            "/  search",
            "v  player",
        ] {
            assert!(footer.contains(hint), "missing `{hint}` in {footer:?}");
        }
    }

    #[test]
    fn wave_page_prompts_for_a_signal_and_renders_the_live_queue() {
        let config = Config::default();
        let mut app = App::demo(&config);
        let actions = app.on_key(KeyEvent::from(KeyCode::Char('w')));
        let [Action::LoadWave { request_id, .. }] = actions.as_slice() else {
            panic!("w must request a wave batch");
        };
        let loading = whole(&render(&app, 80, 24), 80, 24);
        assert!(loading.contains("PERSONAL RADIO"));
        assert!(loading.contains("Tuning your wave"));
        let footer = loading.lines().last().unwrap_or_default();
        assert!(footer.contains("w  wave"));

        app.wave_loaded(*request_id, demo::wave_batch(None));
        let compact = whole(&render(&app, 80, 24), 80, 24);
        assert!(compact.contains("ON AIR"));
        assert!(compact.contains("ENDLESS RADIO"));
        assert!(compact.contains("YOUR WAVE QUEUE"));
        let current = app.current.as_ref().expect("wave track");
        assert!(compact.contains(&current.title));
        let footer = compact.lines().last().unwrap_or_default();
        assert!(footer.contains("enter  play"));
        assert!(footer.contains("n  next"));

        let wide = whole(&render(&app, 200, 60), 200, 60);
        assert!(wide.contains("ON AIR"));
        assert!(wide.contains("YOUR WAVE QUEUE"));
    }

    #[test]
    fn wave_loss_is_visible_and_actionable() {
        let config = Config::default();
        let mut app = App::demo(&config);
        let actions = app.on_key(KeyEvent::from(KeyCode::Char('w')));
        let [Action::LoadWave { request_id, .. }] = actions.as_slice() else {
            panic!("w must request a wave batch");
        };
        app.wave_failed(*request_id, "the network request did not complete".into());
        let text = whole(&render(&app, 80, 24), 80, 24);
        assert!(text.contains("My Wave is off the air"));
        assert!(text.contains("network request"));
        assert!(text.contains("enter  retry"));
    }

    #[test]
    fn now_playing_survives_a_short_terminal_without_losing_metadata() {
        let config = Config::default();
        let mut app = App::demo(&config);
        app.now_playing = true;
        app.focus = Focus::Inspector;
        app.inspector_tab = InspectorTab::Lyrics;

        let text = whole(&render(&app, 80, 24), 80, 24);
        assert!(text.contains("NOW PLAYING"));
        assert!(text.contains("Полярное сияние"));
        assert!(text.contains("Мирабель"));
        assert!(text.contains("Северный свет · 2025"));
        assert!(text.contains("LYRICS"));
        assert!(text.contains("TIME-SYNCED"));
        assert!(text.contains("UP NEXT"));
        let footer = text.lines().last().unwrap_or_default();
        assert!(footer.contains("space  pause"));
        assert!(footer.contains("next/prev"));
        assert!(footer.contains("seek"));
        assert!(footer.contains("back"));
    }

    #[test]
    fn search_and_help_overlays_render_in_a_compact_window() {
        let config = Config::default();
        let mut app = App::demo(&config);
        let query = "ноч";
        app.overlay = Some(Overlay::Search(SearchState {
            query: query.into(),
            focus: SearchFocus::Results,
            results: demo::search(query),
            selected: 0,
            loading: false,
            error: None,
            request_id: 1,
            requested_query: query.into(),
            edited_at: Instant::now(),
        }));
        let text = whole(&render(&app, 80, 24), 80, 24);
        assert!(text.contains("SEARCH"));
        assert!(text.contains("Ночной трамвай"));
        assert!(text.contains("2 RESULTS"));
        assert!(text.contains("esc  input"));

        app.overlay = Some(Overlay::Help);
        let text = whole(&render(&app, 80, 24), 80, 24);
        assert!(text.contains("HELP / KEYBOARD"));
        assert!(text.contains("NAVIGATION"));
        assert!(text.contains("PLAYBACK"));
        assert!(text.contains("INTERFACE"));
        assert!(text.contains("Start or show My Wave"));
        assert!(text.contains("ctrl+c"));
    }

    #[test]
    fn auth_screen_and_tiny_terminal_fallback_render() {
        let config = Config::default();
        let app = App::new(&config, Mode::Live);

        let text = whole(&render(&app, 80, 24), 80, 24);
        assert!(text.contains("SIGN IN"));
        assert!(text.contains("Sign in on this device"));
        assert!(text.contains("Paste an access token"));
        assert!(text.contains("device sign-in"));

        let text = whole(&render(&app, 47, 11), 47, 11);
        assert!(text.contains("too small"));
        assert!(text.contains("48 × 12"));
    }
}
