//! search the catalogue and print what came back.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example search -- "nine inch nails"
//! ```

use yamuse::{Client, SearchQuery, SearchType};

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "nine inch nails".into());

    // the api changes field types without notice; hearing about it beats guessing
    let client = Client::builder()
        .token(token)
        // narrowing a json float to an integer loses nothing; only report real drift
        .on_field_repair(|repair| {
            if repair.is_lossy() {
                eprintln!("note: upstream drift — {repair}");
            }
        })
        .build()?;
    client.init().await?;

    let results = client
        .search(&query, SearchQuery::default().of_type(SearchType::Track))
        .await?;

    if let Some(corrected) = results.correction() {
        println!("(corrected to {corrected:?})\n");
    }

    for track in results.track_results().iter().take(10) {
        let duration = track.duration_ms.unwrap_or(0) / 1000;
        println!(
            "{:>4}:{:02}  {} — {}",
            duration / 60,
            duration % 60,
            track.artists_name(),
            track.title.as_deref().unwrap_or("?"),
        );
    }
    Ok(())
}
