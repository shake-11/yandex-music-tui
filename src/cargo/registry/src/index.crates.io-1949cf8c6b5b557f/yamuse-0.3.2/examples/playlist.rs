//! create a playlist, fill it, rename it and delete it.
//!
//! the point of this one is `revision`. a playlist edit is not "add this track" —
//! it is "apply these operations to the playlist as it was at revision N". the api
//! rejects the edit if anything moved underneath you, and the new revision comes
//! back in the response, so every edit has to read it out and carry it forward.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example playlist
//! ```
//!
//! this one writes to the account. it deletes the playlist it made on the way out.

use yamuse::{Client, DiffTrack, Difference, SearchQuery, Visibility};

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");
    let client = Client::builder().token(token).build()?;
    client.init().await?;

    // something to put in it: the first few results for a search
    let found = client.search("brian eno", SearchQuery::default()).await?;
    let tracks: Vec<DiffTrack> = found
        .track_results()
        .iter()
        .take(5)
        .filter_map(|track| {
            let id = track.id.as_ref()?.as_number()?;
            let album = track.albums.first()?.id?;
            Some(DiffTrack::new(id, album))
        })
        .collect();

    if tracks.is_empty() {
        println!("the search returned nothing usable; nothing to add");
        return Ok(());
    }

    let playlist = client
        .create_playlist("yamuse example", Visibility::Private)
        .await?;
    let kind = playlist.kind.expect("a created playlist always has a kind");
    // revision 1 is what a fresh playlist starts at, but read it rather than assume
    let mut revision = playlist.revision.unwrap_or(1);
    println!("created playlist {kind} at revision {revision}");

    // one operation list, applied in a single request
    let diff = Difference::new().insert(0, tracks.iter().copied());
    let updated = client.change_playlist(kind, &diff, revision).await?;
    revision = updated.revision.unwrap_or(revision);
    println!("added {} tracks, now at revision {revision}", tracks.len());

    for track in &updated.tracks {
        if let Some(track) = track.track.as_ref() {
            println!(
                "  {} — {}",
                track.artists_name(),
                track.title.as_deref().unwrap_or("?")
            );
        }
    }

    // removing is the same mechanism: a half-open range over the current order
    let trimmed = client
        .remove_tracks_from_playlist(kind, 0, 2, revision)
        .await?;
    revision = trimmed.revision.unwrap_or(revision);
    println!("removed the first two, now at revision {revision}");

    // renaming is not a diff, so it needs no revision
    let renamed = client
        .rename_playlist(kind, "yamuse example (renamed)")
        .await?;
    println!("renamed to {:?}", renamed.title.as_deref().unwrap_or("?"));

    // every mutation answers `Mutation`; an api that did not acknowledge the
    // change is an `Err` here, so reaching this line means it really is gone
    client.delete_playlist(kind).await?;
    println!("deleted");

    Ok(())
}
