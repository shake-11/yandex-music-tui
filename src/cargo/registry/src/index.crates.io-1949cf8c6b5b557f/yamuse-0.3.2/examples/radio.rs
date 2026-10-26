//! play a rotor station, reporting feedback as a real client would.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example radio -- user:onyourwave
//! ```

use yamuse::Client;

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let station = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "user:onyourwave".into());

    let client = Client::builder().token(token).build()?;
    client.init().await?;

    let batch = client.rotor_station_tracks(&station, None).await?;
    println!(
        "station {station}, batch {}\n",
        batch.batch_id.as_deref().unwrap_or("?")
    );

    // the rotor personalises from feedback, so a real client always reports it
    client
        .rotor_radio_started(&station, &format!("mobile-radio-{station}"))
        .await?;

    for track in batch.tracks() {
        let id = track.numeric_id()?;
        println!(
            "{} — {}",
            track.artists_name(),
            track.title.as_deref().unwrap_or("?")
        );

        client.rotor_track_started(&station, id).await?;
        // a real player would play the track here; report it as fully listened.
        // a track length fits an i32 with room to spare, so this conversion is exact.
        let seconds = f64::from(i32::try_from(track.duration_ms.unwrap_or(0) / 1000).unwrap_or(0));
        client.rotor_track_finished(&station, id, seconds).await?;
    }
    Ok(())
}
