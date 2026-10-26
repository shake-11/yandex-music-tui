//! find out what the api sends that this crate does not model yet.
//!
//! the models ignore unknown fields on purpose — a private api adds them without
//! notice, and failing on one would be worse. the cost is that a new field is
//! also invisible: you cannot use what you were never told about.
//!
//! this is the other half. it fetches a handful of endpoints raw, parses each one
//! into its model, and reports every key the model did not consume. run it against
//! a real account now and then, or wire it into a staging job, and a new field
//! surfaces the week it appears rather than the year you happen to need it.
//!
//! ```bash
//! YM_TOKEN=... cargo run --example drift
//! ```

use serde::Serialize;
use serde::de::DeserializeOwned;
use yamuse::drift::unmapped_fields;
use yamuse::models::{Album, Artist, Landing, Search, Status, Supplement};
use yamuse::{Client, Params};

/// fetch one endpoint raw, parse it, and report what the model missed.
async fn check<T>(client: &Client, label: &str, path: &str, params: Params)
where
    T: DeserializeOwned + Serialize,
{
    let raw = match client.get_raw(path, params).await {
        Ok(raw) => raw,
        // a 404 here means the sample id is stale, not that anything is wrong
        Err(err) => {
            println!("  {label}: skipped ({err})");
            return;
        },
    };

    // parse the tolerant way, exactly as the typed methods do, so a field whose
    // type drifted is reported as drift rather than failing the whole check
    let parsed: T = match yamuse::tolerant::deserialize_lossy(raw.clone()) {
        Ok(parsed) => parsed,
        Err(err) => {
            println!("  {label}: DOES NOT PARSE — {err}");
            return;
        },
    };

    match unmapped_fields(&raw, &parsed) {
        Ok(fields) if fields.is_empty() => println!("  {label}: fully mapped"),
        Ok(fields) => {
            println!("  {label}: {} unmapped", fields.len());
            // the same new key inside a long list is reported once per element,
            // so collapse the indices to see the distinct shapes
            let mut distinct: Vec<String> =
                fields.iter().map(|field| collapse_indices(field)).collect();
            distinct.sort();
            distinct.dedup();
            for field in distinct.iter().take(20) {
                println!("      {field}");
            }
            if distinct.len() > 20 {
                println!("      … and {} more", distinct.len() - 20);
            }
        },
        Err(err) => println!("  {label}: could not compare — {err}"),
    }
}

/// `blocks[0].entities[2].newField` → `blocks[].entities[].newField`.
fn collapse_indices(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_index = false;
    for c in path.chars() {
        match c {
            '[' => {
                in_index = true;
                out.push('[');
            },
            ']' => {
                in_index = false;
                out.push(']');
            },
            _ if in_index => {},
            c => out.push(c),
        }
    }
    out
}

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let token = std::env::var("YM_TOKEN").expect("set YM_TOKEN");

    let client = Client::builder()
        .token(token)
        // a field whose *type* changed is a different problem from a field this
        // crate never modelled, and this is how you hear about the first kind
        .on_field_repair(|repair| {
            if repair.is_lossy() {
                eprintln!("type drift: {repair}");
            }
        })
        .build()?;
    client.init().await?;

    let uid = client.account_uid().expect("init() populates this");
    println!("checking against account {uid}\n");

    println!("account:");
    check::<Status>(&client, "status", "/account/status", Params::new()).await;

    println!("\ncatalogue:");
    check::<Search>(
        &client,
        "search",
        "/search",
        Params::new()
            .add("text", "radiohead")
            .add("type", "all")
            .add("page", 0)
            .add("nocorrect", false),
    )
    .await;
    check::<Album>(&client, "album", "/albums/1000000", Params::new()).await;
    check::<Artist>(
        &client,
        "artist brief-info",
        "/artists/9262/brief-info",
        Params::new(),
    )
    .await;
    check::<Supplement>(
        &client,
        "track supplement",
        "/tracks/26133163/supplement",
        Params::new(),
    )
    .await;

    println!("\nlanding:");
    check::<Landing>(
        &client,
        "landing3",
        "/landing3",
        Params::new()
            .add_all("blocks", ["personalplaylists", "chart", "new-releases"])
            .add("eitherUserId", "10254713668400548221"),
    )
    .await;

    println!(
        "\nan unmapped field is not a bug — it is a field you could be using.\n\
         add it to the model in src/models/, and it stops showing up here."
    );
    Ok(())
}
