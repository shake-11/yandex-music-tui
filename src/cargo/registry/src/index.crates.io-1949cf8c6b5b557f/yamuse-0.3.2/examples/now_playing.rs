//! watch what the account is playing, on any device.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example now_playing --features realtime
//! ```

use yamuse::Client;
use yamuse::ynison::Event;

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let client = Client::builder().token(token).build()?;

    let realtime = client.realtime()?;
    let mut events = realtime.subscribe();
    println!("watching (device {})...\n", realtime.device_id());

    while let Ok(event) = events.recv().await {
        match event {
            Event::Connected => println!("connected"),
            Event::TrackChanged(playable_id) => {
                // the frame carries only an id; resolve it for a title
                let title = client
                    .track(&playable_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| format!("{} — {}", t.artists_name(), t.title.unwrap_or_default()))
                    .unwrap_or(playable_id);
                println!("▶ {title}");
            },
            Event::PauseChanged(paused) => {
                println!("{}", if paused { "⏸ paused" } else { "▶ resumed" });
            },
            Event::Stale(idle) => println!("(socket idle {idle:?}, reconnecting)"),
            Event::Error(message) => eprintln!("error: {message}"),
            Event::Closed => break,
            _ => {},
        }
    }
    Ok(())
}
