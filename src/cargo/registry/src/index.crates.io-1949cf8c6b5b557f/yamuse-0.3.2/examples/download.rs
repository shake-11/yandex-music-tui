//! download a track, preferring lossless flac.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example download --features download -- 42
//! ```

use yamuse::Client;
use yamuse::download::{Downloader, Strategy};

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let track_id: i64 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "42".into())
        .parse()
        .expect("the track id should be numeric");

    let client = Client::builder().token(token).build()?;
    client.init().await?;

    let Some(track) = client.track(track_id).await? else {
        println!("no such track");
        return Ok(());
    };
    println!(
        "{} — {}",
        track.artists_name(),
        track.title.as_deref().unwrap_or("?")
    );

    let info = client.track_lossless_info(track_id, "lossless").await?;
    let codec = info.codec.as_deref().unwrap_or("?");
    if info.is_lossless() {
        println!("lossless: {codec} @ {} kbps", info.bitrate.unwrap_or(0));
    } else {
        println!("no flac source; the api fell back to {codec}");
    }

    let extension = if info.is_lossless() { "flac" } else { "mp3" };
    let filename = format!("{track_id}.{extension}");

    let written = Downloader::new(client.transport(), info.links())
        .maybe_encraw_key(info.key.clone())
        .strategy(Strategy::Race)
        .to_file(&filename)
        .await?;

    println!("wrote {filename} ({} kb)", written / 1024);
    Ok(())
}
