# Project: yamuse

A typed, async Rust client for the **Yandex Music API** — a private API, not a
published one. The surface is reverse-engineered from the official clients, so
every endpoint's parameters and response shape are observed behaviour rather than
documented contract.

## Stack

- **Rust** edition 2024, MSRV 1.85. `reqwest` over `tokio`, `serde` for models.
- `tokio-tungstenite` for the Ynison WebSocket (feature `realtime`).
- `aes`/`ctr` for `encraw` decryption, `hmac`/`sha2` for request signing, `md-5`
  for the legacy download-link hash.
- No `chrono`: the clock readings and the one iso-8601 format needed are
  hand-rolled in `src/time.rs`.

## Build / Run / Test

```bash
cargo test --all-features        # ~330 tests: unit + doc + integration
cargo clippy --all-features --all-targets   # must be warning-free
cargo fmt                        # rustfmt.toml has nightly-only opts; they no-op on stable
cargo doc --all-features --no-deps
cargo publish --dry-run
```

Integration tests (`tests/`) run against a `wiremock` server — no network, no
token. There are no tests that hit the real API; anything requiring a live token
belongs in `examples/`.

## Layout

- `src/client.rs` — `Client` (cheap `Clone`, shares one pool + account cache) and
  `ClientBuilder`. Also the `pub(crate)` verb helpers every method module calls.
- `src/transport.rs` — HTTP: headers, `{"result": …}` envelope unwrapping, error
  mapping, retry policy.
- `src/api/` — one module per domain, each an `impl Client` block. This is what
  keeps the surface flat (`client.tracks(…)`, `client.search(…)`).
- `src/models/` — serde types. One module per domain, mirroring `api/`.
- `src/sign.rs` — HMAC request signing. `src/crypto.rs` — `encraw` AES-CTR.
- `src/download.rs` (feature `download`) — streaming to disk with mirror
  failover/racing and on-the-fly decryption.
- `src/ynison/` (feature `realtime`) — `state` (frame parsing), `messages`
  (outgoing frames, device identity), `session` (one connection), `realtime`
  (reconnecting client over a broadcast channel).

`examples/` are the runnable documentation — each shows one subsystem end to end
and needs a real token, except `device_auth` which obtains one. `.claude/skills/`
holds the workflows: adding an endpoint, chasing drift, cutting a release.

## Non-obvious constraints

These are load-bearing. Changing them breaks things in ways tests may not catch.

- **The API is camelCase; models are `#[serde(default, rename_all = "camelCase")]`.**
  Every field is `Option` or `Vec` and unknown fields are ignored — a private API
  drops fields without notice and a response must degrade, not fail. Keep this.
  The two device-auth models are the exception: OAuth answers snake_case.
- **Retries are `GET`-only.** A replayed mutation double-applies. Do not
  generalise the retry loop in `transport.rs` to other methods.
- **`get-file-info` signing is order-dependent.** The `codecs` and `transport`
  query values must be byte-identical to what the signature was computed over,
  and the base64 digest has its last character dropped. See `sign.rs`.
- **Ynison frames omit defaults.** betterproto drops `0`, `""`, `false` and
  enum-zero entirely, and sends snake_case while documenting camelCase. Parsing
  accepts both and defaults everything — notably `playback_speed` defaults to
  `1.0`, not `0.0`.
- **The Ynison device id must stay stable across reconnects.** The server
  deduplicates its broadcast by device; a fresh id per reconnect drops the
  observer from the fan-out and state goes silent.
- **Ynison liveness is the ping/pong, not frame arrival.** A paused player is
  legitimately silent for minutes. Tearing the socket down on frame silence
  causes a reconnect storm the server answers with rate limiting.
- **Server "go away" delays are capped** (`go_away_max`, default 60s). Ynison
  answers "temporarily unavailable" with an hour or more; honouring that verbatim
  leaves a watcher dead long after the service returns.
- **Mutation success has two shapes.** Most endpoints answer the bare string
  `"ok"`; track like/dislike answers an object carrying the new `revision`.
  `Client::is_ok_response` handles both.
- **Polymorphic payloads degrade to `Unknown`.** `SearchEntity` and
  `BlockEntityData` keep unrecognised payloads as raw JSON rather than dropping
  them or erroring. One unfamiliar tile must not fail a landing page.
- **`playlists` vs `playlists/list`** are different endpoints with different
  response shapes, and `label*` uses camelCase `pageSize` where `artist*` uses
  hyphenated `page-size`. These inconsistencies are upstream, not typos.

## Conventions

- Comments and docs are **lowercase English**, and explain *why* — a constraint,
  a workaround, an upstream quirk. Never narrate what the code says.
- Every public item has a doc comment (`missing_docs` is `warn`).
- Clippy runs at `pedantic`. Fix warnings rather than allowing them; where a
  numeric cast lint is genuinely inapplicable, use a scoped `#[allow]` **with a
  comment saying why the cast is safe**.
- Test names are sentences describing the property under test
  (`a_dead_mirror_falls_through_to_the_next`), not `test_download_2`.
- Prefer typed enums over stringly-typed parameters; add to `src/types.rs`.
- Options-heavy endpoints get a `*Query`/`*Report` struct with a `Default` that
  matches the API's own defaults, not a long positional signature.

## Commits

**Every commit follows [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).**
No exceptions, including release and documentation commits.

```
<type>(<optional scope>): <subject>

<body: why, not what>

<optional footers>
```

- **Types**: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`,
  `chore`. A behaviour change is `feat` or `fix` even when the diff is small; a
  pure rename or move is `refactor`.
- **Scope** is the module or domain when one applies: `transport`, `download`,
  `ynison`, `models`, `params`, or an endpoint group such as `api/likes`. Omit it
  when the change is genuinely crate-wide.
- **Subject** is lowercase, imperative, no trailing period — the same voice as
  the comments everywhere else here.
- **Breaking changes** get a `!` before the colon *and* a `BREAKING CHANGE:`
  footer saying what a caller has to do about it. This crate is pre-1.0 and
  breaks often; the footer is what makes those findable later, since the version
  number will not distinguish them.
- **The body explains why.** Same rule as the code comments: the diff already
  shows what changed, so the commit message is where the constraint or the
  upstream quirk that forced it gets recorded.
- **Releases** are `chore(release): <version>`, with the changelog entry carrying
  the detail rather than the commit body.

```
fix(download): write through a .part file and rename into place

File::create truncated the destination before the first mirror had answered, so
a failed download destroyed a track that was already there, and a killed process
left a short file under the real name.
```

## Adding an endpoint

1. Model the response in `src/models/<domain>.rs`, `#[serde(default,
   rename_all = "camelCase")]`, every field optional. Add a
   `survives_an_empty_object` case to that module's tests.
2. Add the method to `src/api/<domain>.rs` as `impl Client`, with the HTTP verb
   and path in the doc comment.
3. If it takes more than ~3 parameters, add a query struct and re-export it from
   `src/api/mod.rs` and `src/lib.rs`.
4. Add a `wiremock` test in `tests/transport.rs` asserting the URL, the params
   actually sent, and the response shape.
5. Update the coverage table in `README.md` and `README_ru.md`.
