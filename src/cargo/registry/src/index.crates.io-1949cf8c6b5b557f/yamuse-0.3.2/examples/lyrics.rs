//! print a track's lyrics, time-synced when the api has them.
//!
//! two things worth seeing here. the lyrics endpoint is one of the two that need
//! an hmac signature rather than just the token — the crate signs it for you, but
//! a 401 from this endpoint means the signature, not the token. and the response
//! carries a *url*, not the words, so fetching them is a second request.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example lyrics -- 26133163
//! ```

use yamuse::{Client, Error, LyricsFormat};

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let track_id = std::env::args().nth(1).unwrap_or_else(|| "26133163".into());

    let client = Client::builder().token(token).build()?;
    client.init().await?;

    if let Some(track) = client.track(&track_id).await? {
        println!(
            "{} — {}\n",
            track.artists_name(),
            track.title.as_deref().unwrap_or("?")
        );
    }

    // lrc first: it is the same words plus timestamps, so there is no reason to
    // ask for plain text unless lrc is unavailable
    let lyrics = match client.track_lyrics(&track_id, LyricsFormat::Lrc).await {
        Ok(lyrics) => Some(lyrics),
        // "no lyrics" is a 404 here, which is an answer rather than a failure
        Err(Error::NotFound(_)) => {
            println!("no synced lyrics; trying plain text");
            match client.track_lyrics(&track_id, LyricsFormat::Text).await {
                Ok(lyrics) => Some(lyrics),
                Err(Error::NotFound(_)) => None,
                Err(err) => return Err(err),
            }
        },
        Err(err) => return Err(err),
    };

    let Some(lyrics) = lyrics else {
        println!("this track has no lyrics at all");
        return Ok(());
    };

    if let Some(writers) = Some(&lyrics.writers).filter(|w| !w.is_empty()) {
        println!("written by {}\n", writers.join(", "));
    }

    // the response points at the words rather than carrying them
    let text = lyrics.text(client.transport()).await?;
    println!("{text}");

    Ok(())
}
