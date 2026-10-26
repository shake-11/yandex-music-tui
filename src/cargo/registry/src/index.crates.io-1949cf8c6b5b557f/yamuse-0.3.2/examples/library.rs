//! print the account's library: liked tracks, albums and artists.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example library
//! ```

use yamuse::Client;

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let client = Client::builder().token(token).build()?;
    client.init().await?;

    let status = client.me().expect("init() populates this");
    let login = status
        .account
        .as_ref()
        .and_then(|a| a.login.as_deref())
        .unwrap_or("?");
    println!("{login} (plus: {})\n", status.has_plus());

    let liked = client.liked_tracks(None).await?;
    println!(
        "liked tracks: {} (revision {})",
        liked.tracks.len(),
        liked.revision.unwrap_or(0)
    );

    // the library holds only references; resolve the first few for their titles
    let ids: Vec<String> = liked
        .tracks
        .iter()
        .take(5)
        .filter_map(|t| t.id.as_ref().map(ToString::to_string))
        .collect();
    if !ids.is_empty() {
        for track in client.tracks(&ids).await? {
            println!(
                "  {} — {}",
                track.artists_name(),
                track.title.as_deref().unwrap_or("?")
            );
        }
    }

    let albums = client.liked_albums(None).await?;
    println!("\nliked albums: {}", albums.len());
    for like in albums.iter().take(5) {
        if let Some(album) = &like.album {
            println!(
                "  {} — {}",
                album.artists_name(),
                album.title.as_deref().unwrap_or("?")
            );
        }
    }

    let artists = client.liked_artists(None).await?;
    println!("\nliked artists: {}", artists.len());
    for artist in artists.iter().take(5) {
        println!("  {}", artist.name.as_deref().unwrap_or("?"));
    }
    Ok(())
}
