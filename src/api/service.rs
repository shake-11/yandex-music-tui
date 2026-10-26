//! The asynchronous API layer.
//!
//! `ApiService` is deliberately cloneable: each background request gets its own
//! handle while sharing the underlying HTTP connection pool and cache index.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use yamuse::models::{
    Album as ApiAlbum, Artist as ApiArtist, DownloadInfo, Id, Playlist as ApiPlaylist,
    Track as ApiTrack, TrackShort,
};
use yamuse::{Client, Error as YamuseError, LyricsFormat, SearchQuery};

use crate::cache::AudioCache;
use crate::config::AudioQuality;
use crate::cover::CoverImage;
use crate::lyrics::Lyrics;
use crate::model::{
    AlbumSummary, ArtistSummary, LibraryData, LoadedCollection, PlaylistSummary, SearchResults,
    Track,
};

/// Shared client used by all background operations.
#[derive(Clone)]
pub struct ApiService {
    client: Client,
    http: reqwest::Client,
    cache: AudioCache,
    quality: AudioQuality,
}

/// A partial library plus any endpoint errors. The app keeps the successful
/// portions and schedules a retry rather than hiding a working library.
pub type LibraryLoad = LibraryData;

impl ApiService {
    /// Build the service around an initialised `yamuse` client.
    pub fn new(client: Client, cache: AudioCache, quality: AudioQuality) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "yandex-music-tui/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/shake-11/yandex-music-tui)"
            ))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|err| format!("could not initialise the HTTP client: {err}"))?;
        Ok(Self {
            client,
            http,
            cache,
            quality,
        })
    }

    /// Number of tracks already available offline.
    #[must_use]
    pub fn cached_count(&self) -> usize {
        self.cache.len()
    }

    /// Whether a track is fully cached.
    #[must_use]
    pub fn is_cached(&self, track: &Track) -> bool {
        self.cache.contains(track)
    }

    /// Load every library collection concurrently, preserving partial results.
    pub async fn load_library(&self) -> LibraryLoad {
        let (liked, own_playlists, liked_playlists, artists, albums) = tokio::join!(
            self.client.liked_tracks(None),
            self.client.user_playlists_list(None),
            self.client.liked_playlists(None),
            self.client.liked_artists(None),
            self.client.liked_albums(None),
        );

        let mut data = LibraryLoad::default();
        match liked {
            Ok(list) => match self.resolve_short_tracks(&list.tracks).await {
                Ok(tracks) => data.liked_tracks = tracks,
                Err(err) => data.errors.push(format!("Liked tracks: {err}")),
            },
            Err(err) => data
                .errors
                .push(format!("Liked tracks: {}", friendly_error(&err))),
        }

        let mut seen_playlists = HashSet::new();
        if let Ok(playlists) = own_playlists {
            for playlist in &playlists {
                if let Some(summary) = playlist_summary(playlist) {
                    if seen_playlists.insert(summary.key()) {
                        data.playlists.push(summary);
                    }
                }
            }
        } else if let Err(err) = own_playlists {
            data.errors
                .push(format!("Your playlists: {}", friendly_error(&err)));
        }
        if let Ok(playlists) = liked_playlists {
            for like in &playlists {
                if let Some(playlist) = &like.playlist {
                    if let Some(summary) = playlist_summary(playlist) {
                        if seen_playlists.insert(summary.key()) {
                            data.playlists.push(summary);
                        }
                    }
                }
            }
        } else if let Err(err) = liked_playlists {
            data.errors
                .push(format!("Saved playlists: {}", friendly_error(&err)));
        }

        match artists {
            Ok(artists) => {
                data.artists = artists.iter().filter_map(artist_summary).collect();
            }
            Err(err) => data
                .errors
                .push(format!("Artists: {}", friendly_error(&err))),
        }
        match albums {
            Ok(albums) => {
                data.albums = albums
                    .iter()
                    .filter_map(|like| like.album.as_ref())
                    .filter_map(album_summary)
                    .collect();
            }
            Err(err) => data
                .errors
                .push(format!("Albums: {}", friendly_error(&err))),
        }

        data
    }

    /// Fetch a full playlist and resolve its track references.
    pub async fn load_playlist(
        &self,
        summary: &PlaylistSummary,
    ) -> Result<LoadedCollection, String> {
        let playlist = match self
            .client
            .user_playlist(summary.kind, Some(summary.uid))
            .await
        {
            Ok(playlist) => playlist,
            Err(first) => {
                if let Some(uuid) = &summary.uuid {
                    self.client
                        .playlist_by_uuid(uuid)
                        .await
                        .map_err(|_| friendly_error(&first))?
                } else {
                    return Err(friendly_error(&first));
                }
            }
        };

        let tracks = self.resolve_short_tracks(&playlist.tracks).await?;
        let title = playlist
            .title
            .clone()
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| summary.title.clone());
        let subtitle = playlist_owner(&playlist).unwrap_or_else(|| summary.owner.clone());
        Ok(LoadedCollection {
            title,
            subtitle,
            description: playlist.description.clone(),
            cover_url: playlist_cover(&playlist).or_else(|| summary.cover_url.clone()),
            tracks,
        })
    }

    /// Fetch an artist page and their most popular tracks.
    pub async fn load_artist(&self, summary: &ArtistSummary) -> Result<LoadedCollection, String> {
        let (info, tracks) = tokio::join!(
            self.client.artist(summary.id),
            self.client.artist_tracks(summary.id, 0, 100),
        );
        let tracks = tracks.map_err(|err| friendly_error(&err))?;
        let info = info.ok().flatten();
        let name = info
            .as_ref()
            .and_then(|artist| artist.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| summary.name.clone());
        let cover_url = info
            .as_ref()
            .and_then(|artist| artist.cover_url("600x600"))
            .or_else(|| summary.cover_url.clone());
        let tracks = tracks.tracks.iter().filter_map(track_from_api).collect();
        Ok(LoadedCollection {
            title: name,
            subtitle: "Artist".into(),
            description: info
                .as_ref()
                .and_then(|artist| artist.hand_made_description.clone()),
            cover_url,
            tracks,
        })
    }

    /// Fetch an album with all volumes flattened into queue order.
    pub async fn load_album(&self, summary: &AlbumSummary) -> Result<LoadedCollection, String> {
        let album = self
            .client
            .album_with_tracks(summary.id)
            .await
            .map_err(|err| friendly_error(&err))?;
        let title = album.title.clone().unwrap_or_else(|| summary.title.clone());
        let artist = album.artists_name();
        let tracks = album
            .tracks()
            .into_iter()
            .filter_map(track_from_api)
            .collect();
        Ok(LoadedCollection {
            title,
            subtitle: if artist.is_empty() {
                "Album".into()
            } else {
                artist
            },
            description: album.short_description.clone(),
            cover_url: album
                .cover_url("600x600")
                .or_else(|| summary.cover_url.clone()),
            tracks,
        })
    }

    /// Search the catalogue across all entities.
    pub async fn search(&self, query: &str) -> Result<SearchResults, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchResults::default());
        }

        let search = self
            .client
            .search(query, SearchQuery::default())
            .await
            .map_err(|err| friendly_error(&err))?;

        let tracks = search
            .track_results()
            .iter()
            .take(60)
            .filter_map(track_from_api)
            .collect();
        let albums = search
            .albums
            .as_ref()
            .map(|block| {
                block
                    .results
                    .iter()
                    .take(24)
                    .filter_map(album_summary)
                    .collect()
            })
            .unwrap_or_default();
        let artists = search
            .artists
            .as_ref()
            .map(|block| {
                block
                    .results
                    .iter()
                    .take(20)
                    .filter_map(artist_summary)
                    .collect()
            })
            .unwrap_or_default();
        let playlists = search
            .playlists
            .as_ref()
            .map(|block| {
                block
                    .results
                    .iter()
                    .take(20)
                    .filter_map(playlist_summary)
                    .collect()
            })
            .unwrap_or_default();

        Ok(SearchResults {
            query: search.text.unwrap_or_else(|| query.to_owned()),
            tracks,
            albums,
            artists,
            playlists,
        })
    }

    /// Fetch time-coded lyrics, falling back to plain text.
    ///
    /// `Ok(None)` means the track genuinely has no lyrics; an error means the
    /// request itself failed and is worth surfacing (or retrying).
    pub async fn load_lyrics(&self, track_id: &str) -> Result<Option<Lyrics>, String> {
        let mut lyrics = match self.client.track_lyrics(track_id, LyricsFormat::Lrc).await {
            Ok(lyrics) => lyrics,
            Err(YamuseError::NotFound(_)) => {
                match self.client.track_lyrics(track_id, LyricsFormat::Text).await {
                    Ok(lyrics) => {
                        let text = lyrics
                            .text(self.client.transport())
                            .await
                            .map_err(|err| friendly_error(&err))?;
                        if text.trim().is_empty() {
                            return Ok(None);
                        }
                        let mut parsed = Lyrics::parse_plain(&text);
                        parsed.writers = lyrics.writers;
                        return Ok(Some(parsed));
                    }
                    Err(YamuseError::NotFound(_)) => return Ok(None),
                    Err(err) => return Err(friendly_error(&err)),
                }
            }
            Err(err) => return Err(friendly_error(&err)),
        };

        let writers = std::mem::take(&mut lyrics.writers);
        let text = lyrics
            .text(self.client.transport())
            .await
            .map_err(|err| friendly_error(&err))?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        let mut parsed = Lyrics::parse_lrc(&text);
        parsed.writers = writers;
        Ok(Some(parsed))
    }

    /// Fetch and decode cover art.
    pub async fn load_cover(&self, url: &str) -> Result<CoverImage, String> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|err| format!("cover download failed: {err}"))?
            .error_for_status()
            .map_err(|err| format!("cover download failed: {err}"))?;
        if response
            .content_length()
            .is_some_and(|len| len > 12 * 1024 * 1024)
        {
            return Err("cover image is unreasonably large".into());
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|err| format!("cover download failed: {err}"))?;
        CoverImage::decode(&bytes)
    }

    /// Download a track to the local cache, or return the existing complete file.
    ///
    /// A short-lived direct link is resolved immediately before the transfer.
    /// Downloads land in a unique `.part` file and are atomically renamed.
    pub async fn ensure_cached(&self, track: &Track) -> Result<PathBuf, String> {
        let destination = self.cache.path_for(track);
        if let Ok(metadata) = tokio::fs::metadata(&destination).await {
            if metadata.len() > 16 * 1024 {
                self.cache.mark_complete(track);
                return Ok(destination);
            }
            let _ = tokio::fs::remove_file(&destination).await;
        }
        if !track.available {
            return Err("This track is not available in your region.".into());
        }

        let infos = self
            .client
            .track_download_info(&track.id)
            .await
            .map_err(|err| friendly_error(&err))?;
        let info = choose_mp3(&infos, self.quality)
            .ok_or_else(|| "No MP3 stream is available for this track.".to_owned())?;
        let link = info
            .direct_link(self.client.transport())
            .await
            .map_err(|err| friendly_error(&err))?;

        let response = self
            .http
            .get(link)
            .send()
            .await
            .map_err(|err| format!("audio download failed: {err}"))?
            .error_for_status()
            .map_err(|err| format!("audio download failed: {err}"))?;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let tmp = destination.with_extension(format!("part.{unique}"));
        let write_result = stream_to_file(response, &tmp).await;
        if let Err(err) = write_result {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(err);
        }
        let length = tokio::fs::metadata(&tmp)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if length <= 16 * 1024 {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("The audio stream ended before a complete file arrived.".into());
        }
        tokio::fs::rename(&tmp, &destination)
            .await
            .map_err(|err| format!("could not finish writing the audio cache: {err}"))?;
        self.cache.mark_complete(track);
        Ok(destination)
    }

    /// Add or remove a track like.
    pub async fn set_track_liked(&self, track_id: &str, liked: bool) -> Result<(), String> {
        if liked {
            self.client
                .like_tracks([track_id])
                .await
                .map(|_| ())
                .map_err(|err| friendly_error(&err))
        } else {
            self.client
                .unlike_tracks([track_id])
                .await
                .map(|_| ())
                .map_err(|err| friendly_error(&err))
        }
    }

    async fn resolve_short_tracks(&self, shorts: &[TrackShort]) -> Result<Vec<Track>, String> {
        let mut known: HashMap<String, Track> = HashMap::new();
        for short in shorts {
            if let Some(track) = &short.track {
                if let Some(track) = track_from_api(track) {
                    known.insert(track.id.clone(), track);
                }
            }
        }

        let mut missing = Vec::new();
        let mut missing_seen = HashSet::new();
        for short in shorts {
            let Some(key) = short.id.as_ref().and_then(id_key) else {
                continue;
            };
            if !known.contains_key(&key) && missing_seen.insert(key.clone()) {
                missing.push(key);
            }
        }

        if !missing.is_empty() {
            let fetched = self
                .client
                .tracks(missing)
                .await
                .map_err(|err| friendly_error(&err))?;
            for track in fetched.iter().filter_map(track_from_api) {
                known.insert(track.id.clone(), track);
            }
        }

        Ok(shorts
            .iter()
            .filter_map(|short| short.id.as_ref().and_then(id_key))
            .filter_map(|key| known.get(&key).cloned())
            .collect())
    }
}

/// Choose the MP3 variant closest to the configured target bitrate.
fn choose_mp3(infos: &[DownloadInfo], quality: AudioQuality) -> Option<&DownloadInfo> {
    let target = quality.target_kbps();
    infos
        .iter()
        .filter(|info| info.codec.as_deref() == Some("mp3"))
        .filter(|info| !info.preview.unwrap_or(false))
        .min_by_key(|info| (info.bitrate_in_kbps.unwrap_or(0) - target).abs())
        .or_else(|| {
            infos
                .iter()
                .filter(|info| !info.preview.unwrap_or(false))
                .find(|info| info.codec.as_deref() == Some("mp3"))
        })
}

async fn stream_to_file(response: reqwest::Response, path: &std::path::Path) -> Result<(), String> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|err| format!("could not create the audio cache file: {err}"))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("audio download interrupted: {err}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|err| format!("could not write the audio cache: {err}"))?;
    }
    file.flush()
        .await
        .map_err(|err| format!("could not flush the audio cache: {err}"))?;
    Ok(())
}

fn friendly_error(err: &YamuseError) -> String {
    match err {
        YamuseError::Unauthorized(_) => "the session expired; sign in again".into(),
        YamuseError::NotFound(_) => "not found".into(),
        YamuseError::RateLimited { .. } => "Yandex Music is rate limiting requests".into(),
        YamuseError::Network(_) | YamuseError::TimedOut => {
            "the network request did not complete".into()
        }
        other => other.to_string(),
    }
}

fn id_key(id: &Id) -> Option<String> {
    id.as_number().map(|number| number.to_string())
}

fn track_from_api(track: &ApiTrack) -> Option<Track> {
    let id = track.id.as_ref()?.as_number().map(|id| id.to_string())?;
    let title = track
        .title
        .clone()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Untitled".into());
    let artist = {
        let names = track.artists_name();
        if names.trim().is_empty() {
            "Unknown artist".into()
        } else {
            names
        }
    };
    let album = track.albums.first();
    let album_title = album
        .and_then(|album| album.title.clone())
        .filter(|title| !title.trim().is_empty());
    let year = album.and_then(|album| album.year).or_else(|| {
        album
            .and_then(|album| album.release_date.as_deref())
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok())
    });
    let cover_url = track
        .cover_url("600x600")
        .or_else(|| album.and_then(|album| album.cover_url("600x600")));

    Some(Track {
        id,
        title,
        artist,
        album: album_title,
        year,
        duration_ms: track.duration_ms.unwrap_or(0).max(0) as u64,
        cover_url,
        explicit: track.explicit.unwrap_or(false),
        available: track.available.unwrap_or(true),
    })
}

fn artist_summary(artist: &ApiArtist) -> Option<ArtistSummary> {
    let id = artist.id.as_ref()?.as_number()?;
    let name = artist
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Unknown artist".into());
    Some(ArtistSummary {
        id,
        name,
        cover_url: artist.cover_url("600x600"),
        track_count: artist
            .counts
            .as_ref()
            .and_then(|counts| counts.tracks)
            .and_then(|count| u32::try_from(count).ok()),
    })
}

fn album_summary(album: &ApiAlbum) -> Option<AlbumSummary> {
    let id = album.id?;
    let title = album
        .title
        .clone()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Untitled album".into());
    let artist = {
        let name = album.artists_name();
        if name.trim().is_empty() {
            "Various artists".into()
        } else {
            name
        }
    };
    Some(AlbumSummary {
        id,
        title,
        artist,
        year: album.year,
        track_count: album
            .track_count
            .and_then(|count| u32::try_from(count).ok())
            .unwrap_or(0),
        cover_url: album.cover_url("600x600"),
    })
}

fn playlist_summary(playlist: &ApiPlaylist) -> Option<PlaylistSummary> {
    let kind = playlist.kind?;
    let uid = playlist
        .uid
        .or_else(|| playlist.owner.as_ref().and_then(|owner| owner.uid))?;
    Some(PlaylistSummary {
        uid,
        kind,
        title: playlist
            .title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Untitled playlist".into()),
        owner: playlist_owner(playlist).unwrap_or_else(|| "Yandex Music".into()),
        track_count: playlist
            .track_count
            .and_then(|count| u32::try_from(count).ok())
            .unwrap_or(playlist.tracks.len() as u32),
        duration_ms: playlist.duration_ms.unwrap_or(0).max(0) as u64,
        cover_url: playlist_cover(playlist),
        uuid: playlist.playlist_uuid.clone(),
    })
}

fn playlist_owner(playlist: &ApiPlaylist) -> Option<String> {
    playlist.owner.as_ref().and_then(|owner| {
        owner
            .display_name
            .clone()
            .or_else(|| owner.name.clone())
            .or_else(|| owner.login.clone())
    })
}

fn playlist_cover(playlist: &ApiPlaylist) -> Option<String> {
    playlist
        .cover
        .as_ref()
        .and_then(|cover| cover.url("600x600"))
        .or_else(|| {
            playlist
                .image
                .as_deref()
                .map(|uri| yamuse::models::normalize_image_url(uri, "600x600"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use yamuse::models::DownloadInfo;

    fn download(codec: &str, bitrate: i64, preview: bool) -> DownloadInfo {
        DownloadInfo {
            codec: Some(codec.into()),
            bitrate_in_kbps: Some(bitrate),
            preview: Some(preview),
            ..DownloadInfo::default()
        }
    }

    #[test]
    fn mp3_selection_skips_previews_and_approaches_the_target() {
        let infos = vec![
            download("mp3", 64, false),
            download("aac", 320, false),
            download("mp3", 128, false),
            download("mp3", 320, true),
        ];
        assert_eq!(
            choose_mp3(&infos, AudioQuality::Low).and_then(|info| info.bitrate_in_kbps),
            Some(64)
        );
        assert_eq!(
            choose_mp3(&infos, AudioQuality::Best).and_then(|info| info.bitrate_in_kbps),
            Some(128)
        );
    }

    #[test]
    fn no_mp3_means_no_playable_selection() {
        let infos = vec![download("aac", 320, false)];
        assert!(choose_mp3(&infos, AudioQuality::High).is_none());
    }

    #[test]
    fn playlist_without_ids_is_unactionable() {
        let playlist = ApiPlaylist::default();
        assert!(playlist_summary(&playlist).is_none());
    }

    #[test]
    fn preview_only_variants_are_never_selected() {
        let infos = vec![download("mp3", 320, true)];
        assert!(choose_mp3(&infos, AudioQuality::Best).is_none());
    }
}
