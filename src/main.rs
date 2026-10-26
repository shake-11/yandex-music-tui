//! Command-line entry point for the terminal client.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use yandex_music_tui::runtime::{self, RunOptions};

const HELP: &str = "\
yandex-music-tui — a keyboard-first Yandex Music client

USAGE:
    yandex-music-tui [OPTIONS]

OPTIONS:
    -d, --demo              Explore the interface with a fictional local library
    -l, --login             Start the OAuth device sign-in flow
    -t, --token <TOKEN>     Use an access token for this session
    -c, --config <PATH>     Read and write a specific config.toml
    -h, --help              Show this help
    -V, --version           Show the version

AUTHENTICATION:
    With no saved session, press d for a device code or t to paste a token.
    Tokens can also be supplied through YANDEX_MUSIC_TOKEN or YM_TOKEN.
    Sessions are stored in config.toml with owner-only permissions.

FILES:
    Config    ~/.config/yandex-tui/config.toml
    Cache     ~/.cache/yandex-tui/audio

KEYS:
    j/k move       h/l focus panes     tab cycle panes     / search
    enter play     space pause         n/p next/previous   f like
    d offline      v now playing       i queue/lyrics      ? help
    q quit

EXAMPLES:
    yandex-music-tui --demo
    yandex-music-tui --login
    YANDEX_MUSIC_TOKEN=... yandex-music-tui
";

#[tokio::main]
async fn main() -> ExitCode {
    let options = match parse_args(std::env::args_os().skip(1)) {
        Ok(ParseOutcome::Run(options)) => options,
        Ok(ParseOutcome::Help) => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Ok(ParseOutcome::Version) => {
            println!("yandex-music-tui {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("yandex-music-tui: {message}");
            eprintln!("Try `yandex-music-tui --help` for usage.");
            return ExitCode::from(2);
        }
    };

    match runtime::run(options).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("yandex-music-tui: {err}");
            ExitCode::FAILURE
        }
    }
}

enum ParseOutcome {
    Run(RunOptions),
    Help,
    Version,
}

fn parse_args(args: impl Iterator<Item = OsString>) -> Result<ParseOutcome, String> {
    let mut options = RunOptions::default();
    let mut args = args.peekable();

    while let Some(argument) = args.next() {
        let text = argument.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => return Ok(ParseOutcome::Help),
            "-V" | "--version" => return Ok(ParseOutcome::Version),
            "-d" | "--demo" => options.demo = true,
            "-l" | "--login" => options.login = true,
            "-t" | "--token" => {
                let token = args
                    .next()
                    .ok_or_else(|| "`--token` needs an access token".to_owned())?;
                options.token = Some(
                    token
                        .into_string()
                        .map_err(|_| "the token must be valid UTF-8".to_owned())?,
                );
            }
            "-c" | "--config" => {
                let path = args
                    .next()
                    .ok_or_else(|| "`--config` needs a path".to_owned())?;
                options.config_path = Some(PathBuf::from(path));
            }
            _ if text.starts_with("--token=") => {
                options.token = Some(text["--token=".len()..].to_owned());
            }
            _ if text.starts_with("--config=") => {
                options.config_path = Some(PathBuf::from(&text["--config=".len()..]));
            }
            _ => {
                return Err(format!("unknown option `{text}`"));
            }
        }
    }

    if options.token.as_deref().is_some_and(str::is_empty) {
        return Err("the supplied token is empty".into());
    }
    if options.demo && (options.login || options.token.is_some()) {
        return Err("`--demo` cannot be combined with `--login` or `--token`".into());
    }
    if options.login && options.token.is_some() {
        return Err("choose either `--login` or `--token`".into());
    }

    Ok(ParseOutcome::Run(options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<ParseOutcome, String> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn demo_and_config_flags_are_parsed() {
        let Ok(ParseOutcome::Run(options)) = parse(&["--demo", "--config", "/tmp/ytui.toml"])
        else {
            panic!("expected run options");
        };
        assert!(options.demo);
        assert_eq!(options.config_path, Some(PathBuf::from("/tmp/ytui.toml")));
    }

    #[test]
    fn token_can_be_inline_or_separate() {
        let Ok(ParseOutcome::Run(options)) = parse(&["--token=abc"]) else {
            panic!("expected run options");
        };
        assert_eq!(options.token.as_deref(), Some("abc"));

        let Ok(ParseOutcome::Run(options)) = parse(&["--token", "xyz"]) else {
            panic!("expected run options");
        };
        assert_eq!(options.token.as_deref(), Some("xyz"));
    }

    #[test]
    fn incompatible_modes_are_rejected() {
        assert!(parse(&["--demo", "--login"]).is_err());
        assert!(parse(&["--login", "--token", "abc"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
    }
}
