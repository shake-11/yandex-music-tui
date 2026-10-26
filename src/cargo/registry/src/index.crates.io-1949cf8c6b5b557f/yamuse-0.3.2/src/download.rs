//! streaming downloads.
//!
//! audio files are large — a lossless flac runs to hundreds of megabytes — so the
//! bytes go straight from the socket to disk instead of through a `Vec`. when the
//! stream is `encraw`-encrypted the aes-ctr keystream is applied chunk by chunk on
//! the way past, so decryption costs no extra memory either.
//!
//! `/get-file-info` hands back several mirror urls on different cdn hosts.
//! [`Downloader`] can walk them in order ([`Strategy::Failover`]) or open all of
//! them at once and keep whichever answers first ([`Strategy::Race`]).

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::crypto::EncrawDecryptor;
use crate::error::{Error, Result};
use crate::transport::Transport;

/// how to pick among mirror urls.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// try each url in order, moving on when one fails. the default.
    #[default]
    Failover,
    /// open every url at once and commit to the first that responds, aborting the
    /// rest. cuts tail latency when one cdn host is slow; falls back to
    /// [`Strategy::Failover`] if the winner then breaks mid-download.
    Race,
}

/// a configured download.
///
/// build it with [`Downloader::new`], then run [`to_file`](Downloader::to_file)
/// or [`to_bytes`](Downloader::to_bytes).
pub struct Downloader<'a> {
    transport: &'a Transport,
    urls: Vec<String>,
    key: Option<String>,
    strategy: Strategy,
}

impl<'a> Downloader<'a> {
    /// start a download over the given mirror urls.
    pub fn new(transport: &'a Transport, urls: impl IntoIterator<Item = String>) -> Self {
        Self {
            transport,
            urls: urls.into_iter().collect(),
            key: None,
            strategy: Strategy::default(),
        }
    }

    /// decrypt the stream with this hex `encraw` key as it downloads.
    #[must_use]
    pub fn encraw_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// decrypt with an optional key — convenience for the common
    /// `info.key.clone()` case where encryption may or may not be in play.
    #[must_use]
    pub fn maybe_encraw_key(mut self, key: Option<impl Into<String>>) -> Self {
        self.key = key.map(Into::into);
        self
    }

    /// choose how mirror urls are attempted.
    #[must_use]
    pub fn strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// download to a file on disk, decrypting on the fly when a key is set.
    ///
    /// bytes go to a sibling `.part` file and are renamed into place only once the
    /// transfer has completed, so `path` either holds a whole track or is left
    /// exactly as it was. that matters twice over: a failed download must not
    /// destroy a file already sitting at the destination, and a process killed
    /// mid-transfer must not leave a truncated track that looks like a valid
    /// cached one.
    ///
    /// # errors
    ///
    /// returns the last transport error when every mirror fails, or
    /// [`Error::Io`] when the destination cannot be written.
    pub async fn to_file(&self, path: impl AsRef<Path>) -> Result<u64> {
        let path = path.as_ref();
        self.ensure_urls()?;
        let partial = partial_path(path);

        if self.strategy == Strategy::Race && self.urls.len() > 1 {
            if let Some(written) = self.race_to_file(&partial).await? {
                commit(&partial, path).await?;
                return Ok(written);
            }
            // every racer failed its headers — fall through to the sequential walk
        }

        let mut last = None;
        for url in &self.urls {
            match self.stream_to_file(url, &partial).await {
                Ok(written) => {
                    commit(&partial, path).await?;
                    return Ok(written);
                },
                Err(err) => {
                    remove_partial(&partial).await;
                    last = Some(err);
                },
            }
        }
        Err(last.unwrap_or(Error::IdMissing("download url")))
    }

    /// download into memory, decrypting when a key is set.
    ///
    /// prefer [`to_file`](Self::to_file) for audio; this holds the whole payload.
    /// honours the configured [`Strategy`] the same way `to_file` does.
    ///
    /// # errors
    ///
    /// returns the last transport error when every mirror fails.
    pub async fn to_bytes(&self) -> Result<Vec<u8>> {
        self.ensure_urls()?;

        if self.strategy == Strategy::Race && self.urls.len() > 1 {
            if let Some(response) = self.race_open().await {
                if let Ok(bytes) = collect(response).await {
                    return self.decrypted(bytes);
                }
                // the winner broke mid-stream; fall through to the sequential walk
            }
        }

        let mut last = None;
        for url in &self.urls {
            match self.transport.retrieve(url).await {
                Ok(bytes) => return self.decrypted(bytes),
                Err(err) => last = Some(err),
            }
        }
        Err(last.unwrap_or(Error::IdMissing("download url")))
    }

    /// apply the keystream to a fully-buffered payload, when a key is set.
    fn decrypted(&self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        match &self.key {
            Some(key) => crate::crypto::decrypt_encraw(&bytes, key),
            None => Ok(bytes),
        }
    }

    fn ensure_urls(&self) -> Result<()> {
        if self.urls.is_empty() {
            return Err(Error::IdMissing("download url"));
        }
        Ok(())
    }

    /// open every mirror at once, keep the first to answer, and stream it.
    ///
    /// writes to the `.part` path its caller passes, never to the destination.
    /// returns `Ok(None)` when no mirror produced usable headers, letting the
    /// caller fall back to the sequential walk.
    async fn race_to_file(&self, path: &Path) -> Result<Option<u64>> {
        let Some(response) = self.race_open().await else {
            return Ok(None);
        };
        if let Ok(written) = self.pipe(response, path).await {
            return Ok(Some(written));
        }
        // the winner broke mid-stream: clean up its partial bytes and let the
        // caller retry sequentially.
        remove_partial(path).await;
        Ok(None)
    }

    /// open every mirror at once and return the first to answer headers.
    ///
    /// the losing futures are dropped, which aborts their connections. returns
    /// `None` when every mirror failed to open.
    async fn race_open(&self) -> Option<reqwest::Response> {
        let mut opens = self
            .urls
            .iter()
            .map(|url| {
                let transport = self.transport;
                Box::pin(async move { transport.open_stream(url).await })
            })
            .collect::<Vec<_>>();

        loop {
            if opens.is_empty() {
                return None;
            }
            let (result, _index, rest) = futures_util::future::select_all(opens).await;
            match result {
                Ok(response) => return Some(response),
                Err(_) => opens = rest,
            }
        }
    }

    async fn stream_to_file(&self, url: &str, path: &Path) -> Result<u64> {
        let response = self.transport.open_stream(url).await?;
        self.pipe(response, path).await
    }

    /// write a response body to disk, applying the keystream as it goes.
    async fn pipe(&self, response: reqwest::Response, path: &Path) -> Result<u64> {
        let mut decryptor = match &self.key {
            Some(key) => Some(EncrawDecryptor::from_hex_key(key)?),
            None => None,
        };

        // what the server said it would send, to check against afterwards. this
        // crate never enables response compression, so it describes the bytes that
        // actually arrive.
        let expected = response.content_length();

        // chunks off the wire are small and frequent; every unbuffered write would
        // be a syscall onto tokio's blocking pool.
        let mut file = BufWriter::new(File::create(path).await?);
        let mut stream = response.bytes_stream();
        let mut written = 0u64;

        while let Some(chunk) = stream.next().await {
            let mut chunk = chunk?.to_vec();
            if let Some(decryptor) = &mut decryptor {
                decryptor.apply(&mut chunk);
            }
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;
        }
        file.flush().await?;

        // hyper already rejects a body shorter than its content-length, so this is
        // the backstop rather than the first line of defence — cheap insurance
        // against a transport that does not, since a short flac is
        // indistinguishable from a whole one once it sits under the right name.
        // not a let-chain: those are stable from 1.88 and this crate's msrv is 1.85
        if expected.is_some_and(|expected| written != expected) {
            return Err(Error::InvalidResponse(format!(
                "the download ended early: got {written} bytes of {}",
                expected.unwrap_or_default()
            )));
        }
        Ok(written)
    }
}

/// read a raced response's body into memory.
async fn collect(response: reqwest::Response) -> Result<Vec<u8>> {
    Ok(response.bytes().await?.to_vec())
}

/// drop a half-written file, ignoring the "it was never created" case.
async fn remove_partial(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

/// where the bytes land while the transfer is still in flight.
///
/// a sibling of the destination rather than a temp directory, so the rename that
/// commits it stays within one filesystem and therefore atomic.
fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// move a completed download into place.
async fn commit(partial: &Path, path: &Path) -> Result<()> {
    match tokio::fs::rename(partial, path).await {
        Ok(()) => Ok(()),
        Err(err) => {
            // the destination is unwritable (a directory, no permission, a
            // cross-device path): the download succeeded but has nowhere to go,
            // so report that rather than leaving a stray `.part` behind.
            remove_partial(partial).await;
            Err(Error::Io(err))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failover_is_the_default_strategy() {
        assert_eq!(Strategy::default(), Strategy::Failover);
    }

    #[test]
    fn the_partial_file_is_a_sibling_of_the_destination() {
        // same directory, so the rename that commits it cannot cross a filesystem
        let partial = partial_path(Path::new("/music/track.flac"));
        assert_eq!(partial, PathBuf::from("/music/track.flac.part"));
        assert_eq!(partial.parent(), Path::new("/music/track.flac").parent());
        assert_eq!(partial_path(Path::new("bare")), PathBuf::from("bare.part"));
    }
}
