# yamuse

[![crates.io](https://img.shields.io/crates/v/yamuse.svg)](https://crates.io/crates/yamuse)
[![docs.rs](https://img.shields.io/docsrs/yamuse)](https://docs.rs/yamuse)
[![license](https://img.shields.io/crates/l/yamuse.svg)](LICENSE)

*[по-русски](https://github.com/flxxxxddd/yamuse/blob/main/README_ru.md)*

typed, async rust client for the yandex music api.

covers the catalogue, the library, radio, search, listening history, lossless
downloads and the ynison realtime player stream — the full surface of the private
api the official apps use.

built for the awkward part of that: the api is undocumented and changes without
notice, so every model degrades instead of failing, a mutation that did not happen
is an error rather than a `false`, and a half-written download never reaches disk.

```toml
[dependencies]
yamuse = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

rust 2024, msrv 1.85. no `chrono`, no `openssl` by default.

- [quick start](#quick-start) · [authentication](#authentication) ·
  [downloads](#downloads) · [realtime](#realtime--what-is-playing-right-now)
- [errors](#errors) · [what is covered](#what-is-covered) ·
  [feature flags](#feature-flags) · [design notes](#design-notes)

## quick start

```rust,no_run
use yamuse::Client;

#[tokio::main]
async fn main() -> Result<(), yamuse::Error> {
    let client = Client::builder().token(std::env::var("YM_TOKEN").unwrap()).build()?;
    client.init().await?;

    let results = client.search("nine inch nails", Default::default()).await?;
    for track in results.track_results() {
        println!("{} — {}", track.artists_name(), track.title.as_deref().unwrap_or("?"));
    }
    Ok(())
}
```

## authentication

there is no public api key. authentication is an oauth token, obtained through the
device flow — no browser to embed, no password to handle:

```rust,no_run
# async fn run() -> Result<(), yamuse::Error> {
let client = yamuse::Client::anonymous()?;

let token = client
    .device_auth(Default::default(), |code| {
        println!(
            "open {} and enter {}",
            code.verification_url.as_deref().unwrap_or(""),
            code.user_code.as_deref().unwrap_or(""),
        );
    })
    .await?;

println!("access token: {}", token.access_token.as_deref().unwrap_or(""));
println!("refresh token: {}", token.refresh_token.as_deref().unwrap_or(""));
# Ok(())
# }
```

tokens expire. persist the refresh token and renew with
[`refresh_token`](https://docs.rs/yamuse/latest/yamuse/struct.Client.html#method.refresh_token) —
the response usually rotates it, so store the new one.

## downloads

two paths exist and they are **not** interchangeable.

| | `track_download_info` | `track_lossless_info` |
|---|---|---|
| codecs | mp3, aac | flac, flac-mp4, aac, mp3 |
| endpoint | `/tracks/{id}/download-info` | `/get-file-info` (signed) |
| link lifetime | ~1 minute, resolved from an xml document | mirror urls, longer-lived |
| encryption | none | aes-ctr (`transport: encraw`) |

the lossless path is the one you want, and `Downloader` decrypts it as it streams,
so a 300 mb flac never lands in memory:

```rust,no_run
# #[cfg(feature = "download")]
# async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
use yamuse::download::{Downloader, Strategy};

let info = client.track_lossless_info(42, "lossless").await?;
if !info.is_lossless() {
    eprintln!("no flac source; the api fell back to {:?}", info.codec);
}

Downloader::new(client.transport(), info.links())
    .maybe_encraw_key(info.key.clone())
    .strategy(Strategy::Race) // open every mirror, keep the fastest
    .to_file("track.flac")
    .await?;
# Ok(())
# }
```

## realtime — what is playing right now

ynison is the only server-push channel yandex music offers; there are no webhooks.
`Realtime` owns the handshake, the keepalive and reconnection:

```rust,no_run
# #[cfg(feature = "realtime")]
# async fn run() -> Result<(), yamuse::Error> {
use yamuse::ynison::{Event, Realtime, RealtimeConfig};

let realtime = Realtime::start(RealtimeConfig::new("your-token"));
let mut events = realtime.subscribe();

while let Ok(event) = events.recv().await {
    match event {
        Event::TrackChanged(id) => println!("now playing: {id}"),
        Event::PauseChanged(paused) => println!("{}", if paused { "paused" } else { "playing" }),
        Event::Closed => break,
        _ => {}
    }
}
# Ok(())
# }
```

`realtime.now_playing()` answers synchronously from the last frame, with the
position extrapolated to the moment of the call — useful for serving an http
request without waiting for the next update.

## errors

everything funnels into one [`Error`] enum, so a single `match` covers the whole
surface:

```rust,no_run
# async fn run(client: &yamuse::Client) -> Result<(), yamuse::Error> {
use yamuse::Error;

match client.track(42).await {
    Ok(track) => println!("{track:?}"),
    // the token expired or was never valid
    Err(Error::Unauthorized(_)) => {}
    // the api is throttling; `retry_after()` says for how long, if it said
    Err(Error::RateLimited { retry_after, .. }) => println!("wait {retry_after:?}"),
    Err(err) if err.is_retryable() => println!("transient: {err}"),
    Err(err) => return Err(err),
}
# Ok(())
# }
```

`is_retryable()` is true for transport failures, 5xx and 429. idempotent `GET`s
already retry internally, so seeing one usually means the retries ran out.

[`Error`]: https://docs.rs/yamuse/latest/yamuse/error/enum.Error.html

## what is covered

every endpoint the api exposes, grouped as the client methods are:

| group | endpoints |
|---|---|
| **account** | status, settings (read and write), purchase offers, permission alerts, experiments, promo codes |
| **tracks** | batch lookup, lossy download info, lossless (flac) info, lyrics (plain and lrc), similar, full info, trailer, supplement, playback reporting, after-track shots |
| **albums** | batch lookup, with-tracks, similar entities, trailer |
| **artists** | batch lookup, brief info, tracks, direct/also/discography/safe-direct albums, similar, links, trailer, track ids, info, about, clips, donations, skeletons |
| **playlists** | read, create, delete, rename, visibility, description, diff-based edits, collective join, by uuid, personal (daily and friends), trailer, kinds, similar entities |
| **likes** | like and dislike tracks, artists, albums, playlists and clips; read every library |
| **search** | full search across all types, suggestions |
| **radio** | dashboard, station list, station info, track batches, all four feedback events, station settings |
| **landing** | landing blocks, charts, new releases, new playlists, podcasts, genres, feed, wizard state, tags |
| **history** | listening history by day, item resolution |
| **queues** | list, read, create, position updates |
| **metatags** | navigation trees, metatag detail, albums, artists, playlists |
| **concerts** | artist concerts, concert info, feed, locations, tab config, skeletons |
| **clips** | batch lookup, recommendations |
| **pins** | list, pin and unpin albums, artists, playlists, waves |
| **pre-saves** | list, add, remove |
| **labels** | label detail, albums, artists |
| **credits** | track and clip production credits |
| **disclaimers** | track, clip, album and artist disclaimers |
| **device auth** | code request, polling, refresh, full blocking flow |
| **realtime** | ynison player state, reconnecting |

## feature flags

| flag | default | what it adds |
|---|---|---|
| `rustls-tls` | yes | tls via rustls |
| `native-tls` | no | tls via the platform library instead |
| `download` | yes | stream downloads to disk |
| `realtime` | no | the ynison player stream |
| `tracing` | no | `tracing` events for requests, retries, rate limits and reconnects |
| `full` | no | `download` + `realtime` + `tracing` |

## design notes

**resilience against a private api.** this wraps an api that changes without
notice, so every layer degrades rather than breaks:

- a field the api *stops sending* becomes `None` — every model field is optional.
- a field the api *starts sending* is ignored rather than fatal.
- a field whose *type changes* costs that field, not the request. a json float
  standing in for an integer is narrowed; anything else is dropped, and you are
  told which via `on_field_repair`. this is not hypothetical — `Status.hasOptions`
  reads like a boolean by its very name, and arrives as a list.
- a polymorphic payload with an unrecognized `type` is preserved as raw json
  (`SearchEntity::Unknown`, `BlockEntityData::Unknown`) rather than discarded, so
  one unfamiliar tile never fails a landing page.

```rust,no_run
# fn main() -> Result<(), yamuse::Error> {
let client = yamuse::Client::builder()
    .token("token")
    // narrowing a float loses nothing; is_lossy() filters for real drift
    .on_field_repair(|repair| {
        if repair.is_lossy() {
            eprintln!("api drift: {repair}");
        }
    })
    .build()?;
# Ok(())
# }
```

**retries only where they are safe.** idempotent `GET`s retry transient failures
with exponential backoff and full jitter. mutations get exactly one attempt: a
replayed `playlists/create` would create two playlists. deterministic rejections
(400, 404, 409, 413) are never retried either — they fail the same way every time.

429 is the exception among the 4xx: being rate limited says "not now", not "not
ever". a `retry-after` on the response is waited out verbatim rather than
jittered, but capped at `RetryPolicy::retry_after_max` — this api does answer
with delays of an hour, and honouring one blocks the call for the rest of the day.

**nothing half-written reaches disk.** a download lands in a sibling `.part` file
and is renamed into place only once it completes, so the destination either holds
a whole track or is untouched. that covers both the failed transfer and the
process killed mid-stream — a truncated track looks exactly like a valid cached
one, which is a worse failure than an error.

**typed parameters.** what the api takes as bare strings are real enums here
(`SearchType`, `LyricsFormat`, `MoodEnergy`, ...), so a typo is a compile error
rather than a puzzling 400. anything that reaches the url path is percent-encoded,
so a `/` in a playlist uuid cannot retarget the request at another endpoint.

**a mutation that did not happen is an error.** every mutating method returns
`Mutation`, not `bool`. a boolean could only say "that did not work" — without
the reason, and silently, because nobody checks a returned `bool`. `Mutation`
also carries the library `revision` back from the two endpoints that report one.

**long id lists are split.** a library of several thousand liked tracks would
otherwise be one request the api rejects outright. batch reads chunk transparently;
batch mutations chunk too, and say so, since a failure part-way leaves the earlier
chunks applied.

**cheap clones.** `Client` is `Clone` and shares one connection pool and one
account cache; pass clones around rather than wrapping in another `Arc`.

## examples

| example | what it shows |
|---|---|
| `search` | the shortest useful call, with drift reporting wired up |
| `library` | liked tracks, albums and artists |
| `playlist` | creating and editing one — and why `revision` is unavoidable |
| `lyrics` | a signed endpoint, and a response that carries a url not the words |
| `radio` | a rotor station with the feedback a real player reports |
| `download` | streaming a lossless track to disk, decrypting on the way |
| `now_playing` | the ynison stream |
| `device_auth` | obtaining a token, no browser embedded |
| `drift` | what the api sends that this crate does not model yet |

```bash
YM_TOKEN=... cargo run --example search
YM_TOKEN=... cargo run --example playlist        # writes to the account
YM_TOKEN=... cargo run --example drift
YM_TOKEN=... cargo run --example download --features download
YM_TOKEN=... cargo run --example now_playing --features realtime
cargo run --example device_auth
```

## documentation

- [api reference](https://docs.rs/yamuse) — every method, every model.
- [getting started](https://github.com/flxxxxddd/yamuse/blob/main/docs/getting-started.md) — tokens, the first few calls.
- [architecture](https://github.com/flxxxxddd/yamuse/blob/main/docs/architecture.md) — how the crate is put together, and why
  each load-bearing decision went the way it did.
- [realtime](https://github.com/flxxxxddd/yamuse/blob/main/docs/realtime.md) — the ynison stream in detail.

## alternatives

[`yandex-music`](https://crates.io/crates/yandex-music) is a separate, unrelated
rust crate for the same api — worth evaluating if you want an established
dependency rather than this one.

## legal

this uses a private api, not a published one. the default oauth credentials are
the yandex music android app's, extracted from a publicly downloadable apk;
requests made with them present as that app. that is how every third-party client
for this service works, and it is not endorsed by yandex. check your own terms of
service before deploying anything with it.

## license

MIT
