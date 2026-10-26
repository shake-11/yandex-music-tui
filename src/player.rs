//! A small, thread-owned rodio player.
//!
//! `rodio::OutputStream` is not `Send`, so it never leaves its dedicated audio
//! thread. The UI talks to that thread through commands and receives progress and
//! completion events through a bounded channel. Track ids travel with every event
//! so late events from a previous track can be ignored safely.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use rodio::{Decoder, OutputStream, Sink};

/// Commands accepted by the audio thread.
#[derive(Debug)]
pub enum PlayerCommand {
    /// Decode and play a local MP3.
    Play {
        path: PathBuf,
        track_id: String,
    },
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    Volume(f32),
    Shutdown,
}

/// Events emitted by the audio thread.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// The output device could not be opened. Playback remains unavailable.
    Unavailable(String),
    /// A track started decoding.
    Started(String),
    /// Playback position, in media time.
    Progress {
        track_id: String,
        position: Duration,
    },
    /// The current track reached its end.
    Finished(String),
    /// An operation failed, but the player may still be usable.
    Error {
        track_id: Option<String>,
        message: String,
    },
}

/// Handle to a dedicated audio playback thread.
pub struct AudioPlayer {
    command_tx: Sender<PlayerCommand>,
    event_rx: Receiver<PlayerEvent>,
    thread: Option<thread::JoinHandle<()>>,
}

impl AudioPlayer {
    /// Start the audio thread. Failures are reported as events rather than
    /// aborting construction, so the rest of the interface keeps working.
    #[must_use]
    pub fn spawn(volume: f32) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("ytui-audio".into())
            .spawn(move || audio_main(&command_rx, &event_tx, volume))
            .ok();
        Self {
            command_tx,
            event_rx,
            thread,
        }
    }

    /// Send a command. A dead audio thread is represented by a failed send.
    pub fn send(&self, command: PlayerCommand) -> bool {
        self.command_tx.send(command).is_ok()
    }

    /// Drain pending events without blocking.
    pub fn drain_events(&self, mut handle: impl FnMut(PlayerEvent)) {
        while let Ok(event) = self.event_rx.try_recv() {
            handle(event);
        }
    }

    /// Stop playback and wait briefly for the thread to finish.
    pub fn shutdown(&mut self) {
        let _ = self.command_tx.send(PlayerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PlayerCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn audio_main(
    commands: &Receiver<PlayerCommand>,
    events: &Sender<PlayerEvent>,
    initial_volume: f32,
) {
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(pair) => pair,
        Err(err) => {
            let _ = events.send(PlayerEvent::Unavailable(format!(
                "Audio output is unavailable: {err}"
            )));
            // Consume commands so callers do not block, but do not busy-loop.
            while let Ok(command) = commands.recv() {
                if matches!(command, PlayerCommand::Shutdown) {
                    break;
                }
            }
            return;
        }
    };

    let mut sink: Option<Sink> = None;
    let mut active_id: Option<String> = None;
    let mut volume = initial_volume.clamp(0.0, 1.0);
    let mut last_progress = Instant::now();
    let mut announced_finish = false;

    loop {
        match commands.recv_timeout(Duration::from_millis(120)) {
            Ok(PlayerCommand::Play { path, track_id }) => {
                if let Some(old) = sink.take() {
                    old.stop();
                }
                announced_finish = false;
                match open_sink(&stream_handle, &path) {
                    Ok(new_sink) => {
                        new_sink.set_volume(volume);
                        new_sink.play();
                        sink = Some(new_sink);
                        active_id = Some(track_id.clone());
                        last_progress = Instant::now() - Duration::from_millis(250);
                        let _ = events.send(PlayerEvent::Started(track_id));
                    }
                    Err(message) => {
                        active_id = None;
                        let _ = events.send(PlayerEvent::Error {
                            track_id: Some(track_id),
                            message,
                        });
                    }
                }
            }
            Ok(PlayerCommand::Pause) => {
                if let Some(sink) = &sink {
                    sink.pause();
                }
            }
            Ok(PlayerCommand::Resume) => {
                if let Some(sink) = &sink {
                    sink.play();
                }
            }
            Ok(PlayerCommand::Stop) => {
                if let Some(old) = sink.take() {
                    old.stop();
                }
                active_id = None;
                announced_finish = false;
            }
            Ok(PlayerCommand::Seek(position)) => {
                if let Some(sink) = &sink {
                    if let Err(err) = sink.try_seek(position) {
                        let _ = events.send(PlayerEvent::Error {
                            track_id: active_id.clone(),
                            message: format!("Could not seek in this track: {err}"),
                        });
                    }
                }
            }
            Ok(PlayerCommand::Volume(value)) => {
                volume = value.clamp(0.0, 1.0);
                if let Some(sink) = &sink {
                    sink.set_volume(volume);
                }
            }
            Ok(PlayerCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        if let (Some(active), Some(track_id)) = (&sink, &active_id) {
            if last_progress.elapsed() >= Duration::from_millis(200) {
                let _ = events.send(PlayerEvent::Progress {
                    track_id: track_id.clone(),
                    position: active.get_pos(),
                });
                last_progress = Instant::now();
            }
            if active.empty() && !announced_finish {
                announced_finish = true;
                let _ = events.send(PlayerEvent::Finished(track_id.clone()));
            }
        }
    }
}

fn open_sink(handle: &rodio::OutputStreamHandle, path: &Path) -> Result<Sink, String> {
    let file =
        File::open(path).map_err(|err| format!("Could not open the downloaded audio: {err}"))?;
    let decoder = Decoder::new(BufReader::new(file))
        .map_err(|err| format!("Could not decode the audio file: {err}"))?;
    let sink =
        Sink::try_new(handle).map_err(|err| format!("Could not start audio playback: {err}"))?;
    sink.append(decoder);
    Ok(sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_output_does_not_stall_commands() {
        // This only asserts the command channel contract; whether a device exists
        // depends on the machine running the tests.
        let player = AudioPlayer::spawn(0.5);
        assert!(player.send(PlayerCommand::Volume(0.1)));
        assert!(player.send(PlayerCommand::Stop));
    }
}
