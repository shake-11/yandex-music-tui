# changelog

all notable changes to this crate are documented here. the format follows
[keep a changelog](https://keepachangelog.com/en/1.1.0/), and versioning is
[semver](https://semver.org/spec/v2.0.0.html).

## [0.3.2] — 2026-08-03

### added

- `Client::get_raw` — `GET` any endpoint and get the unwrapped `result` as raw
  json. `drift::unmapped_fields` is public and its whole job is comparing a parsed
  model against the payload it came from, but nothing public could produce that
  payload, so the module was effectively unusable from outside the crate. this is
  also the escape hatch for an endpoint not modelled yet.
- three examples: `playlist` (creating and editing one, and why `revision` is
  unavoidable), `lyrics` (a signed endpoint whose response carries a url rather
  than the words), and `drift` (what the api sends that this crate does not model).

## [0.3.1] — 2026-08-03

documentation only; no code changed, so nothing here affects behaviour.

### added

- `README_ru.md`, a full russian translation rather than a summary, and a
  language switch at the top of both.
- an `errors` section in the readme. `Error` is the first type any caller meets
  and the readme said nothing about it.
- a `documentation` section pointing at docs.rs and the long-form docs. those
  links are absolute: `docs/` is excluded from the published crate, so a relative
  one 404s on crates.io.
- both readmes compile as doctests behind `cfg(doctest)`. an example that stops
  typechecking now fails the build instead of quietly misleading people — which
  matters most for the translation, where drift from the original becomes a
  compile error.

### changed

- the crate is described on its own terms: the api surface is reverse-engineered
  from the official clients, which is what it is. `docs/parity.md` is gone, and
  the doc comments that explained a workaround by pointing at another project's
  type declarations now explain it from the api's own behaviour instead.

## [0.3.0] — 2026-08-03

### added

- `Error::RateLimited`, and with it actual handling of http 429. it is the one
  4xx that retries — being rate limited says "not now", not "not ever". a
  `retry-after` on the response is honoured verbatim rather than jittered, capped
  by the new `RetryPolicy::retry_after_max` (30s). the cap is the lesson ynison's
  `go_away_max` already taught: this api answers with delays of an hour, and
  honouring one leaves a call blocked long after the limit has lifted. only the
  delta-seconds form of the header is read; the http-date form would need a date
  parser this crate does not carry.
- a `tracing` feature, off by default. requests, retries, rate limits and ynison
  reconnects become `tracing` events; without the feature the macros expand to
  nothing. no log line carries a url's query string, because the collective-join
  token and the `get-file-info` signature both travel there.
- `Client::refresh_account` — the status `init` caches never invalidated itself,
  and nothing in a response says it went stale.
- `artist_tracks_all`, `artist_direct_albums_all` and `liked_clips_all` walk every
  page. only the "everything about this one thing" endpoints get these; the
  browse-style listings back infinite-scroll UIs over catalogue-sized result sets,
  where reading every page is a mistake rather than a convenience.
- `BATCH_CHUNK`, and automatic chunking of long id lists. a library of several
  thousand liked tracks was previously one request the api answered with a 413.
  batch reads chunk transparently; batch mutations chunk too, and document that a
  failure part-way leaves the earlier chunks applied.

### fixed

- **a failed download no longer destroys a file already at the destination.**
  bytes went straight to the destination path, so `File::create` truncated it
  before the first mirror had even answered — and if every mirror then failed,
  the cleanup deleted what was left. downloads now land in a sibling `.part` file
  and are renamed into place on success, which also means a process killed
  mid-transfer can no longer leave a truncated track masquerading as a cached one.
- **path parameters are percent-encoded.** a `/`, `?` or `#` in a caller-supplied
  identifier — a playlist uuid, a station tag, a queue id — was interpolated into
  the url raw, so `playlist_by_uuid("../../account/status")` requested a different
  endpoint entirely. `:` and the other rfc 3986 sub-delimiters stay literal,
  because the api's own identifiers use them (`genre:pop`).

### changed

- **breaking.** every mutating method returns `models::Mutation` instead of
  `Result<bool>`, and an unacknowledged mutation is now an `Err`. `Ok(false)` was
  a silent failure: it discarded the reason, and no caller checked it. `Mutation`
  carries the library `revision` that track like/dislike report and that the old
  signature threw away. `let _ = client.like_tracks(..).await?;` is the mechanical
  translation; `if client.like_tracks(..).await?` becomes just the call.
- **breaking.** `RetryPolicy` gained `retry_after_max`; construct it with
  `..RetryPolicy::default()` rather than listing every field.
- **breaking.** the crate root re-exports the `types` enums by name rather than
  with a glob. every existing name is still there; a `use yamuse::*` that relied
  on something else leaking through will need the explicit path.
- downloads write through a `BufWriter` and check the finished size against the
  response's `content-length`.
- `Downloader::to_bytes` honours `Strategy::Race`, which it previously ignored.
- `Realtime` waits on a watch channel instead of polling an `AtomicBool` every
  100 ms, so `stop()` is acted on immediately and a stop that lands between the
  check and the wait can no longer be missed.
- the clock readings and the iso-8601 rendering moved into one internal module
  instead of being spelled out in four places.
- `ClientBuilder::header` documents that it overrides the crate's own headers,
  including the token, and that a repeated name keeps only the last value.
- `Cargo.lock` is committed and CI builds `--locked`; the msrv job runs the tests
  rather than only building, and a `cargo audit` job was added.
- endpoints that split their parameters across the query and the body say so
  through `Params` instead of splicing a `?` into the path. this replaces a
  hand-rolled percent-encoder that only the collective-playlist join used, and
  removes the one place a caller-supplied token reached the query unescaped.
- dropped the unused `url` dependency.

### note on option structs

`SearchQuery`, `PlayAudioReport`, `RotorFeedbackReport`, `DeviceAuthOptions`,
`RealtimeConfig`, `MetatagQuery` and friends stay plain structs with public fields
and a `Default`, deliberately. `#[non_exhaustive]` would forbid the
`..Default::default()` construction they are designed around, in exchange for
making a future field addition non-breaking — a trade this crate does not take.
a new field on one of them is a minor breaking change, and will be called out here.

## [0.2.0] — 2026-08-01

### added

- `tolerant::deserialize` — a response field whose type does not match its model is
  repaired rather than failing the whole call. a json float standing in for an
  integer is narrowed; anything else is dropped. every repair is reported with its
  path through `ClientBuilder::on_field_repair`, and `FieldRepair::is_lossy()`
  distinguishes real drift from harmless number narrowing.
- `models::FlagOrList` — for a field the api answers as either a flag or the list
  it is nominally a flag for.

### changed

- the repair budget counts **distinct fields**, not repairs, and one repair now
  fixes the whole list rather than one element. a field mistyped once in the model
  is mistyped once per element on the wire, so counting raw repairs made a single
  modelling error in a long list look like a wholly unrecognisable payload — and
  failed the very call the budget exists to keep alive. repairing element by
  element would also have cost one deserialize pass each, quadratic in the list
  length. siblings are only touched when they share the offender's json type, so a
  genuinely mixed list still loses only the entries that do not fit.

### fixed

- **breaking.** `Like.id` was `String` and `Artist.id` was `i64`. the live api sends
  a number for album likes (`"id": 38695104`) and a string for artist ones
  (`"id": "4480055"`), so both are now `Id`, which accepts either. this made
  `liked_albums`, `liked_playlists` and `liked_artists` fail outright on any real
  library. `Id` implements `Display`, so `to_string()` callers are unaffected;
  code reading the number wants `as_number()`.
- `Price.amount` was `i64`; the api sends a json number and does use fractions, so
  it is now `f64`.
- `Status.has_options` was `bool`; the live api sends a list. it is now
  `FlagOrList`, which accepts both.

## [0.1.0]

first release: the api surface, modelled and typed.

### added

- **transport** — `{"result": …}` envelope unwrapping, typed errors mapped from
  http status, and jittered exponential backoff on idempotent `GET`s only.
- **request signing** — hmac-sha256 for `tracks/{id}/lyrics` and `get-file-info`,
  including the trailing-character quirk the android client relies on.
- **`encraw` decryption** — aes-ctr (128 and 256), one-shot and streaming.
- **models** — every api payload as a serde type, all fields optional and unknown
  fields ignored, so a changed response degrades rather than fails.
- **endpoints** — all 20 method groups: account, tracks, albums, artists,
  playlists, likes, search, radio, landing, history, queues, metatags, concerts,
  clips, pins, pre-saves, labels, credits, disclaimers, device auth.
- **downloads** (`download`) — streaming to disk with mirror failover or racing,
  decrypting on the fly.
- **realtime** (`realtime`) — ynison player state over websocket, reconnecting,
  with a synchronous `now_playing()` snapshot.
- **typed parameters** — real enums where the api takes bare strings.

### notes

- the default oauth credentials are the yandex music android app's. see the legal
  note in the readme.
- the station restriction the api calls a "value" is `RestrictionValue` here, to
  avoid shadowing `serde_json::Value`.
