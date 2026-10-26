//! Render the real interface with ratatui's test backend and export it as SVG.
//!
//! This is how the screenshots in the README are produced. It never scrapes a
//! terminal: the UI is drawn into an in-memory buffer, then each styled cell is
//! serialised as SVG. Convert the result with `rsvg-convert`:
//!
//! ```sh
//! cargo run --release --example screenshot -- \
//!     --screen now --width 160 --height 45 --out assets/screenshot.svg
//! rsvg-convert -o assets/screenshot.png assets/screenshot.svg
//! ```

use std::env;
use std::error::Error;
use std::fs;
use std::time::Instant;

use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use yandex_music_tui::api::demo;
use yandex_music_tui::app::{App, Focus, Overlay, SearchFocus, SearchState};
use yandex_music_tui::config::Config;
use yandex_music_tui::ui;

const CELL_WIDTH: f32 = 9.0;
const CELL_HEIGHT: f32 = 20.0;
const FONT_SIZE: f32 = 15.0;
const FONT_FAMILY: &str = "JetBrainsMono Nerd Font, JetBrains Mono, monospace";

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let config = Config::default();
    let mut app = match options.screen.as_str() {
        "auth" => {
            let mut app = App::new(&config, yandex_music_tui::app::Mode::Demo);
            app.auth_note = "Sign in to browse your library and play music.".into();
            app
        }
        _ => App::demo(&config),
    };

    match options.screen.as_str() {
        "now" => {
            app.now_playing = true;
            app.focus = Focus::Inspector;
            app.inspector_tab = yandex_music_tui::app::InspectorTab::Lyrics;
        }
        "home" => {
            app.section = yandex_music_tui::app::Section::Home;
            app.view = yandex_music_tui::app::View::Library(yandex_music_tui::app::Section::Home);
            app.nav_index = 0;
            app.focus = Focus::Main;
            app.main_selected = 1;
        }
        "liked" => {
            app.section = yandex_music_tui::app::Section::Liked;
            app.view = yandex_music_tui::app::View::Library(yandex_music_tui::app::Section::Liked);
            app.nav_index = 1;
            app.focus = Focus::Main;
            app.main_selected = 0;
        }
        "search" => {
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
        }
        "help" => app.overlay = Some(Overlay::Help),
        "auth" => {}
        other => return Err(format!("unknown screen `{other}`").into()),
    }

    let backend = TestBackend::new(options.width, options.height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| ui::draw(frame, &app))?;
    let buffer = terminal.backend().buffer();

    if options.dump {
        let mut text = String::new();
        for y in 0..options.height {
            for x in 0..options.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        print!("{text}");
    }

    let svg = render_svg(buffer, options.width, options.height);
    if let Some(parent) = options.out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&options.out, svg)?;
    eprintln!(
        "wrote {} ({}×{} cells)",
        options.out.display(),
        options.width,
        options.height
    );
    Ok(())
}

struct Options {
    screen: String,
    width: u16,
    height: u16,
    out: std::path::PathBuf,
    dump: bool,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            screen: "now".into(),
            width: 160,
            height: 45,
            out: "assets/screenshot.svg".into(),
            dump: false,
        };
        let mut args = env::args().skip(1);
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--screen" => options.screen = next_value(&mut args, "--screen")?,
                "--width" => options.width = next_value(&mut args, "--width")?.parse()?,
                "--height" => options.height = next_value(&mut args, "--height")?.parse()?,
                "--out" => options.out = next_value(&mut args, "--out")?.into(),
                "--dump" => options.dump = true,
                "--help" | "-h" => {
                    println!(
                        "usage: screenshot [--screen now|home|liked|search|help|auth] \
                         [--width N] [--height N] [--out PATH.svg] [--dump]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument `{other}`").into()),
            }
        }
        Ok(options)
    }
}

fn next_value(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, Box<dyn Error>> {
    args.next()
        .ok_or_else(|| format!("`{option}` needs a value").into())
}

fn render_svg(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    let pixel_width = f32::from(width) * CELL_WIDTH;
    let pixel_height = f32::from(height) * CELL_HEIGHT;
    let mut svg = String::with_capacity(usize::from(width) * usize::from(height) * 24);
    svg.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixel_width:.0}\" \
         height=\"{pixel_height:.0}\" viewBox=\"0 0 {pixel_width:.0} {pixel_height:.0}\">\n\
         <rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
        colour(theme_bg())
    ));

    // Background rectangles, merged horizontally to keep the file lean.
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let background = buffer[(x, y)].bg;
            let start = x;
            while x < width && buffer[(x, y)].bg == background {
                x += 1;
            }
            if background != theme_bg() && background != Color::Reset {
                svg.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\"/>\n",
                    f32::from(start) * CELL_WIDTH,
                    f32::from(y) * CELL_HEIGHT,
                    f32::from(x - start) * CELL_WIDTH,
                    CELL_HEIGHT,
                    colour(background)
                ));
            }
        }
    }

    // Text runs, split only when colour or weight changes.
    for y in 0..height {
        let mut x = 0;
        while x < width {
            let cell = &buffer[(x, y)];
            let foreground = cell.fg;
            let modifier = cell.modifier;
            let start = x;
            let mut run = String::new();
            while x < width {
                let current = &buffer[(x, y)];
                if current.fg != foreground || current.modifier != modifier {
                    break;
                }
                run.push_str(current.symbol());
                x += 1;
            }
            let trimmed_start = run.len() - run.trim_start_matches(' ').len();
            let trimmed = run.trim_matches(' ');
            if !trimmed.is_empty() && !trimmed.chars().all(char::is_control) {
                let start_chars = run[..trimmed_start].chars().count();
                let x = f32::from(start) + start_chars as f32;
                svg.push_str(&text_run(
                    x * CELL_WIDTH,
                    f32::from(y) * CELL_HEIGHT + CELL_HEIGHT * 0.78,
                    trimmed,
                    foreground,
                    modifier,
                ));
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn text_run(x: f32, y: f32, text: &str, foreground: Color, modifier: Modifier) -> String {
    let mut font = String::new();
    if modifier.contains(Modifier::BOLD) {
        font.push_str(" font-weight=\"700\"");
    }
    if modifier.contains(Modifier::ITALIC) {
        font.push_str(" font-style=\"italic\"");
    }
    if modifier.contains(Modifier::UNDERLINED) {
        font.push_str(" text-decoration=\"underline\"");
    }
    let opacity = if modifier.contains(Modifier::DIM) {
        " opacity=\"0.65\""
    } else {
        ""
    };
    format!(
        "<text x=\"{x:.1}\" y=\"{y:.1}\" font-family=\"{FONT_FAMILY}\" \
         font-size=\"{FONT_SIZE}\" fill=\"{}\"{font}{opacity} \
         xml:space=\"preserve\">{}</text>\n",
        colour(foreground),
        escape(text)
    )
}

fn theme_bg() -> Color {
    Color::Rgb(9, 9, 10)
}

fn colour(color: Color) -> String {
    match color {
        Color::Reset => "#f4f2eb".into(),
        Color::Black => "#1a1a1c".into(),
        Color::Red => "#ff716a".into(),
        Color::Green => "#7cd99d".into(),
        Color::Yellow => "#ffcc00".into(),
        Color::Blue => "#93b7f6".into(),
        Color::Magenta => "#cba6f7".into(),
        Color::Cyan => "#89dceb".into(),
        Color::Gray => "#a6a6a6".into(),
        Color::DarkGray => "#545458".into(),
        Color::LightRed => "#ffa198".into(),
        Color::LightGreen => "#a6e3a1".into(),
        Color::LightYellow => "#ffe273".into(),
        Color::LightBlue => "#b4c7fa".into(),
        Color::LightMagenta => "#e0b0ff".into(),
        Color::LightCyan => "#b5e8f0".into(),
        Color::White => "#f4f2eb".into(),
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
        Color::Indexed(index) => {
            if index < 16 {
                colour(indexed(index))
            } else if index < 232 {
                let value = index - 16;
                let red = (value / 36) * 51;
                let green = ((value % 36) / 6) * 51;
                let blue = (value % 6) * 51;
                format!("#{red:02x}{green:02x}{blue:02x}")
            } else {
                let level = 8 + (index - 232) * 10;
                format!("#{level:02x}{level:02x}{level:02x}")
            }
        }
    }
}

fn indexed(index: u8) -> Color {
    match index {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        _ => Color::White,
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
