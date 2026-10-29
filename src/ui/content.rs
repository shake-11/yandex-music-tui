//! The middle pane: home, saved collections, and detailed playlist/artist/album
//! views. Everything here is responsive to the pane width rather than the whole
//! terminal, so a wide screen with a narrow content column still reads cleanly.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, CollectionRequest, CollectionView, Focus, Section, View, WaveState};
use crate::model::{
    AlbumSummary, ArtistSummary, LibraryData, LoadedCollection, PlaylistSummary, Track,
};
use crate::ui::components;
use crate::ui::icons;
use crate::ui::{cover, primitives, text, theme};

/// Draw the middle content pane for the current view.
pub fn draw(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if area.width < 10 || area.height < 4 {
        return;
    }
    primitives::paint(frame, area, theme::base());
    match &app.view {
        View::Library(Section::Home) => draw_home(frame, app, area),
        View::Library(Section::Wave) => draw_wave(frame, app, area),
        View::Library(Section::Liked) => draw_liked(frame, app, area),
        View::Library(section) => draw_saved(frame, app, area, *section),
        View::Collection(collection) => draw_collection(frame, app, area, collection),
    }
}

struct Preamble {
    title_y: u16,
    content_y: u16,
    inner: Rect,
    compact: bool,
}

fn preamble(
    frame: &mut Frame<'_>,
    area: Rect,
    eyebrow: &str,
    title: &str,
    subtitle: &str,
    right: Option<&str>,
) -> Preamble {
    let pad = if area.width >= 40 { 2 } else { 1 };
    let inner = Rect::new(
        area.x + pad,
        area.y,
        area.width.saturating_sub(pad * 2),
        area.height,
    );
    let compact = area.height < 9 || area.width < 30;
    let title_y = if compact { area.y } else { area.y + 2 };
    if !compact {
        components::eyebrow(
            frame,
            Rect::new(inner.x, area.y + 1, inner.width, 1),
            eyebrow,
        );
        if let Some(right) = right {
            primitives::right_line(
                frame,
                Rect::new(inner.x, area.y + 1, inner.width, 1),
                vec![Span::styled(
                    right.to_owned(),
                    Style::default().fg(theme::FAINT),
                )],
                theme::base(),
            );
        }
    }
    primitives::line(
        frame,
        Rect::new(inner.x, title_y, inner.width, 1),
        vec![Span::styled(
            text::truncate(title, usize::from(inner.width)),
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::BG)
                .add_modifier(Modifier::BOLD),
        )],
        theme::base(),
    );
    let subtitle_y = title_y + 1;
    if subtitle_y < area.y + area.height {
        primitives::line(
            frame,
            Rect::new(inner.x, subtitle_y, inner.width, 1),
            vec![Span::styled(
                text::truncate(subtitle, usize::from(inner.width)),
                Style::default().fg(theme::MUTED).bg(theme::BG),
            )],
            theme::base(),
        );
    }
    let content_y = if compact { title_y + 2 } else { area.y + 5 };
    Preamble {
        title_y,
        content_y,
        inner,
        compact,
    }
}

fn library_problem(frame: &mut Frame<'_>, app: &App, area: Rect, extra: &str) -> bool {
    if let Some(library) = &app.library {
        if let Some(error) = app.content_error() {
            if area.height >= 7 {
                let y = area.y + 4;
                primitives::line(
                    frame,
                    Rect::new(area.x + 2, y, area.width.saturating_sub(4), 1),
                    vec![
                        Span::styled(
                            format!("{} ", icons::WARNING),
                            Style::default().fg(theme::RED),
                        ),
                        Span::styled(
                            text::truncate(error, usize::from(area.width.saturating_sub(5))),
                            Style::default().fg(theme::RED),
                        ),
                    ],
                    theme::base(),
                );
            }
        }
        let _ = library;
        return false;
    }
    if app.library_loading {
        components::message_panel(
            frame,
            area,
            components::spinner(app.animation_tick),
            "Building your library",
            extra,
            theme::YELLOW,
        );
        return true;
    }
    if app.library_error.is_some() {
        components::message_panel(
            frame,
            area,
            icons::WARNING,
            "Your library is offline",
            "Check the connection and press r to retry now. Automatic retries are already scheduled with exponential backoff.",
            theme::RED,
        );
        return true;
    }
    components::message_panel(
        frame,
        area,
        icons::HEADPHONES,
        "Nothing here yet",
        "Start listening in Yandex Music and your library will appear here.",
        theme::MUTED,
    );
    true
}

fn draw_home(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(library) = &app.library else {
        let _ = preamble(
            frame,
            area,
            "YOUR LIBRARY",
            "Home",
            "Everything you saved, in one quiet place.",
            None,
        );
        library_problem(
            frame,
            app,
            area,
            "Fetching liked tracks, playlists, artists, and albums. Network failures retry automatically.",
        );
        return;
    };

    let total_tracks = library.liked_tracks.len();
    let total_duration: u64 = library
        .liked_tracks
        .iter()
        .map(|track| track.duration_ms)
        .sum();
    let title = format!("Good evening, {}", first_name(app));
    let subtitle = if total_tracks == 0 {
        "Your library is ready. Save a few tracks to start a collection.".to_owned()
    } else {
        format!(
            "{} liked tracks · {} of listening",
            total_tracks,
            text::total_time(total_duration)
        )
    };
    let heading = preamble(
        frame,
        area,
        "YOUR LIBRARY",
        &title,
        &subtitle,
        Some(&format!("{} PLAYLISTS", library.playlists.len())),
    );

    let mut y = if heading.compact {
        heading.title_y + 2
    } else {
        heading.content_y
    };
    let stats_height = draw_stats(frame, app, heading.inner, y, library);
    y += stats_height + 1;

    if y + 2 >= area.y + area.height {
        return;
    }
    let list_area = Rect::new(
        heading.inner.x,
        y,
        heading.inner.width,
        area.y + area.height - y,
    );
    let header_y = list_area.y;
    primitives::line(
        frame,
        Rect::new(list_area.x, header_y, list_area.width, 1),
        vec![
            Span::styled(
                "RECENTLY LIKED",
                Style::default()
                    .fg(theme::FAINT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", icons::DOT),
                Style::default().fg(theme::YELLOW_DIM),
            ),
        ],
        theme::base(),
    );
    primitives::right_line(
        frame,
        Rect::new(list_area.x, header_y, list_area.width, 1),
        vec![Span::styled(
            if app.focus == Focus::Main {
                "ENTER TO PLAY"
            } else {
                "LIKED TRACKS"
            },
            Style::default().fg(theme::FAINT),
        )],
        theme::base(),
    );

    let tracks = &library.liked_tracks;
    let rows = Rect::new(
        list_area.x,
        header_y + 1,
        list_area.width,
        list_area.height.saturating_sub(1),
    );
    if tracks.is_empty() {
        components::message_panel(
            frame,
            rows,
            icons::HEART_O,
            "No liked tracks yet",
            "Press f on a track to save it here.",
            theme::MUTED,
        );
    } else {
        draw_track_rows(frame, app, rows, tracks);
    }
}

fn draw_wave(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let ready = app.wave.as_ref().is_some_and(|wave| wave.ready);
    let subtitle = if ready {
        "An endless stream shaped by what you actually listen to."
    } else {
        "Your own station. No playlist, no final track."
    };
    let heading = preamble(
        frame,
        area,
        "PERSONAL RADIO",
        "My Wave",
        subtitle,
        Some("∞ ENDLESS"),
    );
    let content_y = if heading.compact {
        heading.title_y + 2
    } else {
        heading.content_y
    };
    let content = Rect::new(
        heading.inner.x,
        content_y,
        heading.inner.width,
        area.y + area.height - content_y,
    );
    if content.height == 0 {
        return;
    }

    let Some(wave) = &app.wave else {
        draw_wave_prompt(
            frame,
            content,
            icons::WAVE,
            "Your wave is waiting",
            "A personal radio stream that keeps finding the next track. Start it once and let it run.",
            ("enter", "tune in"),
            theme::YELLOW,
        );
        return;
    };

    if !wave.ready {
        if wave.loading {
            draw_wave_prompt(
                frame,
                content,
                components::spinner(app.animation_tick),
                "Tuning your wave",
                "Finding a first batch of tracks from your listening profile. This keeps retrying if the signal drops.",
                ("esc", "sidebar"),
                theme::YELLOW,
            );
        } else if let Some(error) = &wave.error {
            draw_wave_prompt(
                frame,
                content,
                icons::WARNING,
                "My Wave is off the air",
                &format!("{error}. Automatic retries are scheduled; press enter to try again now."),
                ("enter", "retry"),
                theme::RED,
            );
        } else {
            draw_wave_prompt(
                frame,
                content,
                icons::WAVE,
                "Your wave is waiting",
                "Press enter to start an endless personal stream. Press w from anywhere to return to it.",
                ("enter", "tune in"),
                theme::YELLOW,
            );
        }
        return;
    }

    draw_wave_live(frame, app, content, wave);
}

fn draw_wave_live(frame: &mut Frame<'_>, app: &App, area: Rect, wave: &WaveState) {
    if area.height == 0 {
        return;
    }
    let bottom = area.y + area.height;
    let mut y = area.y;
    let right = if wave.loading {
        format!("{}  TUNING NEXT", components::spinner(app.animation_tick))
    } else if wave.error.is_some() {
        format!("{}  SIGNAL LOST", icons::WARNING)
    } else {
        format!("∞  {} QUEUED", app.queue.len())
    };
    let right_width = text::width(&right);
    let left_width = area.width.saturating_sub(right_width as u16 + 2);
    let status = vec![
        Span::styled(
            " ON AIR ",
            Style::default()
                .fg(theme::BG)
                .bg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ENDLESS RADIO",
            Style::default().fg(theme::MUTED).bg(theme::BG),
        ),
    ];
    primitives::line(
        frame,
        Rect::new(area.x, y, left_width, 1),
        status,
        theme::base(),
    );
    primitives::right_line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![Span::styled(
            right,
            Style::default()
                .fg(if wave.error.is_some() {
                    theme::RED
                } else if wave.loading {
                    theme::YELLOW_SOFT
                } else {
                    theme::FAINT
                })
                .bg(theme::BG),
        )],
        theme::base(),
    );
    y += 1;

    if let Some(track) = &app.current {
        if y < bottom {
            let title_room = usize::from(area.width).saturating_sub(18);
            let title = text::truncate(&track.title, title_room);
            let used = 5 + text::width(&title) + 5;
            let artist_room = usize::from(area.width).saturating_sub(used);
            let artist = if artist_room >= 4 {
                text::truncate(&track.artist, artist_room)
            } else {
                String::new()
            };
            let mut spans = vec![
                Span::styled(
                    "NOW  ",
                    Style::default()
                        .fg(theme::YELLOW_DIM)
                        .bg(theme::BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(title, Style::default().fg(theme::TEXT).bg(theme::BG)),
            ];
            if !artist.is_empty() {
                spans.push(Span::styled(
                    "  ·  ",
                    Style::default().fg(theme::FAINT).bg(theme::BG),
                ));
                spans.push(Span::styled(
                    artist,
                    Style::default().fg(theme::MUTED).bg(theme::BG),
                ));
            }
            primitives::line(
                frame,
                Rect::new(area.x, y, area.width, 1),
                spans,
                theme::base(),
            );
            y += 1;
        }
    }

    if let Some(error) = &wave.error {
        if y < bottom {
            let suffix = "  ·  r retry";
            let reserved = 3 + text::width(suffix) + 1;
            primitives::line(
                frame,
                Rect::new(area.x, y, area.width, 1),
                vec![
                    Span::styled(
                        format!("{}  ", icons::WARNING),
                        Style::default().fg(theme::RED).bg(theme::BG),
                    ),
                    Span::styled(
                        text::truncate(
                            error,
                            usize::from(area.width.saturating_sub(reserved as u16)),
                        ),
                        Style::default().fg(theme::RED).bg(theme::BG),
                    ),
                    Span::styled(suffix, Style::default().fg(theme::FAINT).bg(theme::BG)),
                ],
                theme::base(),
            );
            y += 1;
        }
    }

    y += 1;
    if y >= bottom {
        return;
    }
    primitives::line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![
            Span::styled(
                "YOUR WAVE QUEUE",
                Style::default()
                    .fg(theme::FAINT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default().bg(theme::BG)),
            Span::styled(icons::DOT, Style::default().fg(theme::YELLOW_DIM)),
        ],
        theme::base(),
    );
    primitives::right_line(
        frame,
        Rect::new(area.x, y, area.width, 1),
        vec![Span::styled(
            if app.focus == Focus::Main {
                "ENTER TO PLAY"
            } else {
                "MY WAVE"
            },
            Style::default().fg(theme::FAINT),
        )],
        theme::base(),
    );
    y += 1;

    let rows = Rect::new(area.x, y, area.width, bottom.saturating_sub(y));
    if app.queue.is_empty() {
        components::message_panel(
            frame,
            rows,
            icons::WAVE,
            "The station is between tracks",
            "The next batch is being tuned. It will start automatically.",
            theme::MUTED,
        );
    } else {
        draw_track_rows(frame, app, rows, &app.queue);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_wave_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    icon: &str,
    title: &str,
    message: &str,
    hint: (&str, &str),
    colour: ratatui::style::Color,
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
            text::truncate(title, usize::from(box_area.width.saturating_sub(7))),
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD),
        )],
        Style::default().bg(theme::PANEL),
    );
    if box_area.height >= 6 {
        let text_area = Rect::new(
            box_area.x + 5,
            box_area.y + 3,
            box_area.width.saturating_sub(7),
            box_area.height.saturating_sub(5),
        );
        frame.render_widget(
            Paragraph::new(message.to_owned())
                .style(Style::default().fg(theme::MUTED).bg(theme::PANEL))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .alignment(Alignment::Left),
            text_area,
        );
        primitives::line(
            frame,
            Rect::new(
                box_area.x + 5,
                box_area.y + box_area.height - 2,
                box_area.width.saturating_sub(7),
                1,
            ),
            vec![
                components::keycap(hint.0),
                Span::styled(
                    format!(" {}", hint.1),
                    Style::default().fg(theme::FAINT).bg(theme::PANEL),
                ),
            ],
            Style::default().bg(theme::PANEL),
        );
    }
}

fn first_name(app: &App) -> String {
    let name = app.profile.short_name().trim();
    if name.is_empty() {
        "listener".into()
    } else {
        name.split_whitespace().next().unwrap_or(name).to_owned()
    }
}

fn draw_stats(frame: &mut Frame<'_>, app: &App, area: Rect, y: u16, library: &LibraryData) -> u16 {
    let stats = [
        ("LIKED", library.liked_tracks.len()),
        ("PLAYLISTS", library.playlists.len()),
        ("ARTISTS", library.artists.len()),
        ("ALBUMS", library.albums.len()),
    ];
    let mut y = y;
    if area.width >= 54 {
        let mut spans = Vec::new();
        for (index, (label, value)) in stats.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled("     ", Style::default().bg(theme::BG)));
            }
            spans.push(Span::styled(
                format!("{value:>2} "),
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                label.to_owned(),
                Style::default().fg(theme::MUTED).bg(theme::BG),
            ));
        }
        primitives::line(
            frame,
            Rect::new(area.x, y, area.width, 1),
            spans,
            theme::base(),
        );
        return 1;
    }

    for pair in stats.chunks(2) {
        let mut spans = Vec::new();
        for (index, (label, value)) in pair.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled("       ", Style::default().bg(theme::BG)));
            }
            spans.push(Span::styled(
                format!("{value:>2} "),
                Style::default()
                    .fg(theme::YELLOW)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                label.to_owned(),
                Style::default().fg(theme::MUTED).bg(theme::BG),
            ));
        }
        if y < area.y + area.height {
            primitives::line(
                frame,
                Rect::new(area.x, y, area.width, 1),
                spans,
                theme::base(),
            );
        }
        y += 1;
    }
    // Keep the layout stable even when the pane is unusually short.
    let _ = app;
    2
}

fn draw_liked(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let Some(library) = &app.library else {
        let _ = preamble(
            frame,
            area,
            "COLLECTION",
            "Liked tracks",
            "Tracks you saved, newest first.",
            None,
        );
        library_problem(
            frame,
            app,
            area,
            "Fetching your liked tracks. Failures retry automatically with exponential backoff.",
        );
        return;
    };
    let total_duration: u64 = library
        .liked_tracks
        .iter()
        .map(|track| track.duration_ms)
        .sum();
    let subtitle = format!(
        "{} tracks · {}",
        library.liked_tracks.len(),
        text::total_time(total_duration)
    );
    let heading = preamble(
        frame,
        area,
        "COLLECTION",
        "Liked tracks",
        &subtitle,
        Some(&format!("{} TRACKS", library.liked_tracks.len())),
    );
    let content_y = if heading.compact {
        heading.title_y + 2
    } else {
        heading.content_y
    };
    let content = Rect::new(
        heading.inner.x,
        content_y,
        heading.inner.width,
        area.y + area.height - content_y,
    );
    if library.liked_tracks.is_empty() {
        components::message_panel(
            frame,
            content,
            icons::HEART_O,
            "No liked tracks yet",
            "Press f on any track to add it to this collection.",
            theme::MUTED,
        );
    } else {
        draw_track_rows(frame, app, content, &library.liked_tracks);
    }
}

fn draw_saved(frame: &mut Frame<'_>, app: &App, area: Rect, section: Section) {
    let Some(library) = &app.library else {
        let _ = preamble(
            frame,
            area,
            section.eyebrow(),
            section.title(),
            &section_subtitle(&section, None),
            None,
        );
        library_problem(
            frame,
            app,
            area,
            "Fetching your saved collections. Failures retry automatically.",
        );
        return;
    };

    let count = match section {
        Section::Playlists => library.playlists.len(),
        Section::Artists => library.artists.len(),
        Section::Albums => library.albums.len(),
        Section::Home | Section::Wave | Section::Liked => 0,
    };
    let heading = preamble(
        frame,
        area,
        section.eyebrow(),
        section.title(),
        &section_subtitle(&section, Some(count)),
        Some(&format!("{count} SAVED")),
    );
    let content_y = if heading.compact {
        heading.title_y + 2
    } else {
        heading.content_y
    };
    let content = Rect::new(
        heading.inner.x,
        content_y,
        heading.inner.width,
        area.y + area.height - content_y,
    );

    match section {
        Section::Playlists => {
            if library.playlists.is_empty() {
                empty_collection(
                    frame,
                    content,
                    icons::PLAYLIST,
                    "No playlists yet",
                    "Playlists you create or save in Yandex Music will be listed here.",
                );
            } else {
                draw_card_rows(frame, app, content, &library.playlists);
            }
        }
        Section::Artists => {
            if library.artists.is_empty() {
                empty_collection(
                    frame,
                    content,
                    icons::ARTIST,
                    "No artists yet",
                    "Artists you like in Yandex Music will be collected here.",
                );
            } else {
                draw_card_rows(frame, app, content, &library.artists);
            }
        }
        Section::Albums => {
            if library.albums.is_empty() {
                empty_collection(
                    frame,
                    content,
                    icons::ALBUM,
                    "No albums yet",
                    "Albums you save from the catalogue will appear here.",
                );
            } else {
                draw_card_rows(frame, app, content, &library.albums);
            }
        }
        Section::Home | Section::Wave | Section::Liked => {}
    }
}

fn section_subtitle(section: &Section, count: Option<usize>) -> String {
    match section {
        Section::Playlists => match count {
            Some(count) => format!("{count} sets you made or saved, ready to play."),
            None => "Sets you made or saved, ready to play.".into(),
        },
        Section::Artists => match count {
            Some(count) => format!("{count} voices you keep coming back to."),
            None => "Voices you keep coming back to.".into(),
        },
        Section::Albums => match count {
            Some(count) => format!("{count} records worth returning to."),
            None => "Records worth returning to.".into(),
        },
        Section::Home | Section::Wave | Section::Liked => String::new(),
    }
}

fn empty_collection(frame: &mut Frame<'_>, area: Rect, icon: &str, title: &str, message: &str) {
    components::message_panel(frame, area, icon, title, message, theme::MUTED);
}

trait Card {
    fn card_title(&self) -> &str;
    fn card_subtitle(&self) -> String;
    fn card_seed(&self) -> String;
}

impl Card for PlaylistSummary {
    fn card_title(&self) -> &str {
        &self.title
    }

    fn card_subtitle(&self) -> String {
        let duration = text::total_time(self.duration_ms);
        format!(
            "{} · {} tracks · {}",
            self.owner, self.track_count, duration
        )
    }

    fn card_seed(&self) -> String {
        format!("{} / {}", self.title, self.owner)
    }
}

impl Card for ArtistSummary {
    fn card_title(&self) -> &str {
        &self.name
    }

    fn card_subtitle(&self) -> String {
        match self.track_count {
            Some(count) if count > 0 => format!("{count} tracks in your library"),
            _ => "Artist".to_owned(),
        }
    }

    fn card_seed(&self) -> String {
        format!("artist / {}", self.name)
    }
}

impl Card for AlbumSummary {
    fn card_title(&self) -> &str {
        &self.title
    }

    fn card_subtitle(&self) -> String {
        let year = self
            .year
            .map_or_else(String::new, |year| format!(" · {year}"));
        format!("{}{} · {} tracks", self.artist, year, self.track_count)
    }

    fn card_seed(&self) -> String {
        format!("{} / {}", self.title, self.artist)
    }
}

fn draw_card_rows<T: Card>(frame: &mut Frame<'_>, app: &App, area: Rect, items: &[T]) {
    if area.height == 0 || items.is_empty() {
        return;
    }
    let visible = (usize::from(area.height) / 2).max(1);
    let start = text::window_start(app.main_selected, items.len(), visible);
    let row_width = area.width;
    let text_x = area.x + 9;
    let text_width = row_width.saturating_sub(11);
    for (offset, item) in items.iter().enumerate().skip(start).take(visible) {
        let index = offset;
        let y = area.y + (index - start) as u16 * 2;
        let selected = index == app.main_selected;
        let background = if selected {
            if app.focus == Focus::Main {
                theme::SELECT_FOCUS
            } else {
                theme::SELECT
            }
        } else {
            theme::BG
        };
        let row = Rect::new(area.x, y, row_width, 2);
        primitives::paint(frame, row, Style::default().bg(background));
        if selected {
            primitives::vertical_rule(
                frame,
                area.x,
                y,
                2,
                Style::default().fg(theme::YELLOW).bg(background),
            );
        }
        let tile = Rect::new(area.x + 2, y, 4, 2);
        cover::draw_tile(frame, tile, &item.card_seed(), selected);
        let title_style = Style::default()
            .fg(if selected {
                theme::TEXT
            } else {
                theme::TEXT_SOFT
            })
            .bg(background)
            .add_modifier(Modifier::BOLD);
        primitives::line(
            frame,
            Rect::new(text_x, y, text_width, 1),
            vec![Span::styled(
                text::truncate(item.card_title(), usize::from(text_width)),
                title_style,
            )],
            Style::default().bg(background),
        );
        primitives::line(
            frame,
            Rect::new(text_x, y + 1, text_width, 1),
            vec![Span::styled(
                text::truncate(
                    &item.card_subtitle(),
                    usize::from(text_width.saturating_sub(2)),
                ),
                Style::default().fg(theme::MUTED).bg(background),
            )],
            Style::default().bg(background),
        );
        if selected {
            primitives::right_line(
                frame,
                Rect::new(area.x + area.width.saturating_sub(3), y, 2, 1),
                vec![Span::styled(
                    "›",
                    Style::default()
                        .fg(theme::YELLOW)
                        .bg(background)
                        .add_modifier(Modifier::BOLD),
                )],
                Style::default().bg(background),
            );
        }
    }
}

fn draw_collection(frame: &mut Frame<'_>, app: &App, area: Rect, collection: &CollectionView) {
    let pad = if area.width >= 40 { 2 } else { 1 };
    let inner = Rect::new(
        area.x + pad,
        area.y,
        area.width.saturating_sub(pad * 2),
        area.height,
    );
    let Some(data) = &collection.data else {
        let subtitle = format!(
            "{} · loading from your library",
            collection.request.kind_label()
        );
        let heading = preamble(
            frame,
            area,
            collection.request.kind_label(),
            &collection.request.title(),
            &subtitle,
            None,
        );
        let content_y = if heading.compact {
            heading.title_y + 2
        } else {
            heading.content_y
        };
        let content = Rect::new(
            inner.x,
            content_y,
            inner.width,
            area.y + area.height - content_y,
        );
        if let Some(error) = &collection.error {
            components::message_panel(
                frame,
                content,
                icons::WARNING,
                "Could not load this collection",
                error,
                theme::RED,
            );
        } else {
            components::message_panel(
                frame,
                content,
                components::spinner(app.animation_tick),
                "Loading collection",
                "Fetching the track list from Yandex Music.",
                theme::YELLOW,
            );
        }
        return;
    };

    let hero_height = if area.height >= 30 {
        10
    } else if area.height >= 19 {
        8
    } else {
        5
    }
    .min(area.height.saturating_sub(3));
    let hero = Rect::new(inner.x, area.y + 1, inner.width, hero_height);
    let show_cover = inner.width >= 62 && hero_height >= 6;
    let cover_width = if show_cover { 16 } else { 0 };
    if show_cover {
        let cover_area = Rect::new(hero.x, hero.y, cover_width, hero_height);
        cover::draw_placeholder(
            frame,
            cover_area,
            &data.title,
            data.subtitle.split_whitespace().next().unwrap_or("MUSIC"),
        );
    }
    let text_x = hero.x + if show_cover { cover_width + 3 } else { 0 };
    let text_width = inner
        .width
        .saturating_sub(if show_cover { cover_width + 3 } else { 0 });
    let mut y = hero.y;
    primitives::line(
        frame,
        Rect::new(text_x, y, text_width, 1),
        vec![
            Span::styled(
                format!("{} ", collection.request.kind_label()),
                Style::default()
                    .fg(theme::YELLOW_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} TRACKS", data.tracks.len()),
                Style::default().fg(theme::FAINT),
            ),
        ],
        theme::base(),
    );
    y += 1;
    if y < hero.y + hero.height {
        primitives::line(
            frame,
            Rect::new(text_x, y, text_width, 1),
            vec![Span::styled(
                text::truncate(&data.title, usize::from(text_width)),
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            )],
            theme::base(),
        );
        y += 1;
    }
    let total_duration: u64 = data.tracks.iter().map(|track| track.duration_ms).sum();
    if y < hero.y + hero.height {
        let mut subtitle = data.subtitle.clone();
        if !subtitle.is_empty() {
            subtitle.push_str(" · ");
        }
        subtitle.push_str(&format!(
            "{} tracks · {}",
            data.tracks.len(),
            text::total_time(total_duration)
        ));
        if let Some(error) = &collection.error {
            subtitle = format!("{} · retrying: {error}", subtitle);
        }
        primitives::line(
            frame,
            Rect::new(text_x, y, text_width, 1),
            vec![Span::styled(
                text::truncate(&subtitle, usize::from(text_width)),
                Style::default().fg(theme::MUTED).bg(theme::BG),
            )],
            theme::base(),
        );
        y += 2;
    }
    if let Some(description) = &data.description {
        let remaining = (hero.y + hero.height).saturating_sub(y);
        let lines = text::wrap(description, usize::from(text_width));
        for line in lines.into_iter().take(usize::from(remaining)) {
            if y >= hero.y + hero.height {
                break;
            }
            primitives::line(
                frame,
                Rect::new(text_x, y, text_width, 1),
                vec![Span::styled(
                    line,
                    Style::default().fg(theme::TEXT_SOFT).bg(theme::BG),
                )],
                theme::base(),
            );
            y += 1;
        }
    }

    let list_y = hero.y + hero.height + 1;
    if list_y >= area.y + area.height {
        return;
    }
    let list_area = Rect::new(inner.x, list_y, inner.width, area.y + area.height - list_y);
    primitives::line(
        frame,
        Rect::new(list_area.x, list_area.y, list_area.width, 1),
        vec![
            Span::styled(
                "TRACKS",
                Style::default()
                    .fg(theme::FAINT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", icons::DOT),
                Style::default().fg(theme::YELLOW_DIM),
            ),
        ],
        theme::base(),
    );
    let playing = app
        .current
        .as_ref()
        .and_then(|current| data.tracks.iter().position(|track| track.id == current.id));
    if let Some(index) = playing {
        primitives::right_line(
            frame,
            Rect::new(list_area.x, list_area.y, list_area.width, 1),
            vec![
                Span::styled("NOW PLAYING  ", Style::default().fg(theme::YELLOW_DIM)),
                Span::styled(
                    format!("{:02}", index + 1),
                    Style::default()
                        .fg(theme::YELLOW)
                        .add_modifier(Modifier::BOLD),
                ),
            ],
            theme::base(),
        );
    }
    let rows = Rect::new(
        list_area.x,
        list_area.y + 1,
        list_area.width,
        list_area.height.saturating_sub(1),
    );
    if data.tracks.is_empty() {
        components::message_panel(
            frame,
            rows,
            icons::HEADPHONES,
            "This collection is empty",
            "There are no playable tracks here yet.",
            theme::MUTED,
        );
    } else {
        draw_track_rows(frame, app, rows, &data.tracks);
    }
}

fn draw_track_rows(frame: &mut Frame<'_>, app: &App, area: Rect, tracks: &[Track]) {
    if area.height == 0 || tracks.is_empty() || area.width < 12 {
        return;
    }
    let visible = usize::from(area.height);
    let start = text::window_start(app.main_selected, tracks.len(), visible);
    let row_width = area.width;
    for (offset, track) in tracks.iter().enumerate().skip(start).take(visible) {
        let index = offset;
        let y = area.y + (index - start) as u16;
        let selected = index == app.main_selected;
        let current = app
            .current
            .as_ref()
            .is_some_and(|current| current.id == track.id);
        let background = if selected {
            if app.focus == Focus::Main {
                theme::SELECT_FOCUS
            } else {
                theme::SELECT
            }
        } else if current {
            theme::PANEL
        } else {
            theme::BG
        };
        let row = Rect::new(area.x, y, row_width, 1);
        primitives::paint(frame, row, Style::default().bg(background));
        let marker = if selected {
            Span::styled(
                "▌",
                Style::default()
                    .fg(if app.focus == Focus::Main {
                        theme::YELLOW
                    } else {
                        theme::YELLOW_DIM
                    })
                    .bg(background),
            )
        } else {
            Span::styled(" ", Style::default().bg(background))
        };
        let number_style = Style::default()
            .fg(if selected {
                theme::YELLOW_DIM
            } else {
                theme::FAINT
            })
            .bg(background);
        let status = if current {
            if app.playing {
                Span::styled("▶ ", Style::default().fg(theme::YELLOW).bg(background))
            } else {
                Span::styled("▮ ", Style::default().fg(theme::YELLOW_DIM).bg(background))
            }
        } else if track.available {
            Span::styled("  ", Style::default().bg(background))
        } else {
            Span::styled("× ", Style::default().fg(theme::RED).bg(background))
        };
        let number = format!("{:>2} ", index + 1);
        let duration = if row_width >= 28 {
            text::duration(track.duration())
        } else {
            String::new()
        };
        let duration_width = usize::from(if duration.is_empty() {
            0
        } else {
            row_width.min(7)
        });
        let prefix_width = 1 + 2 + 1 + 2; // marker, number, space, status
        let text_width = usize::from(row_width).saturating_sub(prefix_width + duration_width);
        let title_full = text::truncate(&track.title, text_width.saturating_sub(2));
        let used = text::width(&title_full);
        let artist_room = text_width.saturating_sub(used + 2);
        let artist = if artist_room >= 4 {
            format!("  {}", text::truncate(&track.artist, artist_room - 2))
        } else {
            String::new()
        };
        let title_style = Style::default()
            .fg(if current {
                theme::YELLOW_SOFT
            } else if selected || track.available {
                theme::TEXT
            } else {
                theme::MUTED
            })
            .bg(background)
            .add_modifier(if selected || current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        let mut spans = vec![
            marker,
            Span::styled(number, number_style),
            status,
            Span::styled(title_full, title_style),
            Span::styled(artist, Style::default().fg(theme::MUTED).bg(background)),
        ];
        if !duration.is_empty() {
            let used = primitives::spans_width(&spans);
            let pad = usize::from(row_width).saturating_sub(used + text::width(&duration) + 1);
            spans.push(Span::styled(
                " ".repeat(pad),
                Style::default().bg(background),
            ));
            spans.push(Span::styled(
                duration,
                Style::default()
                    .fg(if selected {
                        theme::TEXT_SOFT
                    } else {
                        theme::FAINT
                    })
                    .bg(background),
            ));
            spans.push(Span::styled(" ", Style::default().bg(background)));
        }
        primitives::line(frame, row, spans, Style::default().bg(background));
    }
}

/// A one-line collection summary, reused by the now-playing screen.
#[must_use]
pub fn collection_summary(data: &LoadedCollection) -> String {
    let duration: u64 = data.tracks.iter().map(|track| track.duration_ms).sum();
    format!(
        "{} tracks · {}",
        data.tracks.len(),
        text::total_time(duration)
    )
}

/// A human label for a collection request.
#[must_use]
pub fn request_label(request: &CollectionRequest) -> &'static str {
    request.kind_label()
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::config::Config;

    #[test]
    fn every_library_section_renders_without_panicking() {
        let mut app = App::demo(&Config::default());
        for section in Section::ALL {
            app.section = section;
            app.view = View::Library(section);
            app.main_selected = 1;
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| draw(frame, &app, frame.area()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert!(buffer
                .content
                .iter()
                .any(|cell| !cell.symbol().trim().is_empty()));
        }
    }
}
