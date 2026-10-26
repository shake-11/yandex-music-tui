//! Self-contained data for `--demo`, tests, and the README screenshot.
//!
//! Demo content is original and fictional. It exercises long Cyrillic titles,
//! mixed Latin metadata, a queue, and time-coded lyrics without touching the
//! network or requiring credentials.

use std::time::Duration;

use crate::cover::CoverImage;
use crate::lyrics::Lyrics;
use crate::model::{
    AlbumSummary, ArtistSummary, LibraryData, PlaylistSummary, Profile, SearchResults, Track,
};

/// A small, deliberately varied library.
#[must_use]
pub fn library() -> LibraryData {
    LibraryData {
        profile: Profile {
            login: "night.listener".into(),
            display_name: "Alex".into(),
            has_plus: true,
        },
        liked_tracks: tracks(),
        playlists: playlists(),
        artists: artists(),
        albums: albums(),
        errors: Vec::new(),
    }
}

/// Demo tracks in the order shown in the library.
#[must_use]
pub fn tracks() -> Vec<Track> {
    vec![
        track(
            "10000001",
            "Полярное сияние",
            "Мирабель",
            "Северный свет",
            Some(2025),
            252_000,
            true,
        ),
        track(
            "10000002",
            "Velvet Static",
            "Low Season",
            "Soft Machines",
            Some(2024),
            228_000,
            false,
        ),
        track(
            "10000003",
            "Ночной трамвай",
            "Кинотеатр",
            "Огни",
            Some(2023),
            276_000,
            false,
        ),
        track(
            "10000004",
            "Glass Horizon",
            "Aster Vale",
            "Tidal",
            Some(2025),
            302_000,
            false,
        ),
        track(
            "10000005",
            "Тихий снег",
            "Поля",
            "Зима",
            Some(2022),
            201_000,
            false,
        ),
        track(
            "10000006",
            "Afterimage",
            "Mono Lake",
            "Drift",
            Some(2024),
            247_000,
            false,
        ),
        track(
            "10000007",
            "Между строк",
            "Сигнал",
            "Полутона",
            Some(2025),
            235_000,
            false,
        ),
        track(
            "10000008",
            "Blue Hour",
            "Sundial",
            "Low Tide",
            Some(2023),
            284_000,
            false,
        ),
        track(
            "10000009",
            "Северный ветер",
            "Океан",
            "Берег",
            Some(2021),
            371_000,
            false,
        ),
        track(
            "10000010",
            "Satellite Hearts",
            "Nova June",
            "Signals",
            Some(2025),
            213_000,
            false,
        ),
        track(
            "10000011",
            "Пыль",
            "Тишина",
            "Комнаты",
            Some(2022),
            178_000,
            false,
        ),
        track(
            "10000012",
            "Last Train Home",
            "Paper Planes",
            "Night Routes",
            Some(2024),
            259_000,
            false,
        ),
    ]
}

fn track(
    id: &str,
    title: &str,
    artist: &str,
    album: &str,
    year: Option<i64>,
    duration_ms: u64,
    explicit: bool,
) -> Track {
    Track {
        id: id.into(),
        title: title.into(),
        artist: artist.into(),
        album: Some(album.into()),
        year,
        duration_ms,
        cover_url: None,
        explicit,
        available: true,
    }
}

/// Fictional playlists for the sidebar and library view.
#[must_use]
pub fn playlists() -> Vec<PlaylistSummary> {
    vec![
        PlaylistSummary {
            uid: 777_001,
            kind: 1,
            title: "Ночная поездка".into(),
            owner: "Alex".into(),
            track_count: 38,
            duration_ms: 9_840_000,
            cover_url: None,
            uuid: None,
        },
        PlaylistSummary {
            uid: 777_001,
            kind: 2,
            title: "Deep Focus".into(),
            owner: "Alex".into(),
            track_count: 62,
            duration_ms: 15_660_000,
            cover_url: None,
            uuid: None,
        },
        PlaylistSummary {
            uid: 777_001,
            kind: 3,
            title: "Русский инди".into(),
            owner: "Alex".into(),
            track_count: 24,
            duration_ms: 6_240_000,
            cover_url: None,
            uuid: None,
        },
        PlaylistSummary {
            uid: 777_001,
            kind: 4,
            title: "Late Night Synths".into(),
            owner: "Alex".into(),
            track_count: 31,
            duration_ms: 8_010_000,
            cover_url: None,
            uuid: None,
        },
        PlaylistSummary {
            uid: 777_001,
            kind: 5,
            title: "Утро без спешки".into(),
            owner: "Alex".into(),
            track_count: 19,
            duration_ms: 4_980_000,
            cover_url: None,
            uuid: None,
        },
    ]
}

/// Fictional artists represented in the demo library.
#[must_use]
pub fn artists() -> Vec<ArtistSummary> {
    let mut seen = std::collections::HashSet::new();
    tracks()
        .into_iter()
        .filter_map(|track| {
            if !seen.insert(track.artist.clone()) {
                return None;
            }
            let id = track.id.parse::<i64>().unwrap_or(1) + 500_000;
            Some(ArtistSummary {
                id,
                name: track.artist,
                cover_url: None,
                track_count: Some(1),
            })
        })
        .collect()
}

/// Fictional albums represented in the demo library.
#[must_use]
pub fn albums() -> Vec<AlbumSummary> {
    let mut seen = std::collections::HashSet::new();
    tracks()
        .into_iter()
        .filter_map(|track| {
            let album = track.album?;
            if !seen.insert(album.clone()) {
                return None;
            }
            let id = track.id.parse::<i64>().unwrap_or(1) + 900_000;
            Some(AlbumSummary {
                id,
                title: album,
                artist: track.artist,
                year: track.year,
                track_count: 1,
                cover_url: None,
            })
        })
        .collect()
}

/// The current track used by the demo and screenshot.
#[must_use]
pub fn current_track() -> Track {
    tracks().remove(0)
}

/// A queue that starts with the current track.
#[must_use]
pub fn queue() -> Vec<Track> {
    tracks().into_iter().take(5).collect()
}

/// Time-coded lyrics for the first demo track. Original text.
#[must_use]
pub fn lyrics() -> Lyrics {
    Lyrics::parse_lrc(
        "\
[00:00.00]Под полярным светом
[00:11.50]город спит в снегу
[00:24.00]Я иду по линии огней
[00:37.50]к северному берегу
[00:50.00]Ветер помнит наши имена
[01:03.00]и рисует их на льду
[01:17.00]Если ты услышишь этот звон
[01:30.00]я тебя найду
[01:44.00]Полярное сияние
[01:56.00]рассыпается в темноте
[02:09.00]Мы останемся здесь до утра
[02:22.00]на самой высокой черте
[02:38.00]Снег ложится на провода
[02:51.00]и стирает все пути
[03:05.00]Но пока горит эта звезда
[03:18.00]нам некуда идти
[03:33.00]Под полярным светом
[03:48.00]город спит в снегу
",
    )
}

/// Demo lyrics for the opening track; other demo tracks are intentionally
/// instrumental so the empty state is exercised too.
#[must_use]
pub fn lyrics_for(track: &Track) -> Option<Lyrics> {
    if track.id == "10000001" {
        Some(lyrics())
    } else {
        None
    }
}

/// A deterministic generated cover, so demo mode looks complete offline.
#[must_use]
pub fn cover(track: &Track) -> CoverImage {
    CoverImage::procedural(&format!("{} / {}", track.title, track.artist))
}

/// Perform a simple case-insensitive local search over the demo library.
#[must_use]
pub fn search(query: &str) -> SearchResults {
    let needle = query.trim().to_lowercase();
    let library = library();
    if needle.is_empty() {
        return SearchResults::default();
    }
    let matches = |value: &str| value.to_lowercase().contains(&needle);
    SearchResults {
        query: query.trim().to_owned(),
        tracks: library
            .liked_tracks
            .into_iter()
            .filter(|track| {
                matches(&track.title)
                    || matches(&track.artist)
                    || track.album.as_deref().is_some_and(matches)
            })
            .collect(),
        albums: library
            .albums
            .into_iter()
            .filter(|album| matches(&album.title) || matches(&album.artist))
            .collect(),
        artists: library
            .artists
            .into_iter()
            .filter(|artist| matches(&artist.name))
            .collect(),
        playlists: library
            .playlists
            .into_iter()
            .filter(|playlist| matches(&playlist.title))
            .collect(),
    }
}

/// Demo progress used in a paused first-run state.
#[must_use]
pub const fn start_position() -> Duration {
    Duration::from_secs(107)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_library_is_rich_enough_to_exercise_the_ui() {
        let library = library();
        assert!(library.liked_tracks.len() >= 10);
        assert!(library.playlists.len() >= 4);
        assert!(library.artists.len() >= 5);
        assert!(library.albums.len() >= 5);
    }

    #[test]
    fn demo_lyrics_are_synchronised() {
        let lyrics = lyrics();
        assert!(lyrics.synced);
        assert!(lyrics
            .active_index(start_position(), Duration::from_secs(252))
            .is_some());
    }

    #[test]
    fn local_search_finds_cyrillic_titles() {
        let results = search("ночной");
        assert!(!results.tracks.is_empty());
    }
}
