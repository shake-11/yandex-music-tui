//! transport behaviour against a mock server.
//!
//! the unit tests cover envelope and error parsing in isolation; these check the
//! parts only a real http round-trip exercises — headers actually sent, query and
//! form encoding, status mapping, and that retries happen exactly where intended.

use std::time::Duration;

use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use yamuse::{Client, Error, RetryPolicy};

/// a client pointed at the mock server, with retries off unless a test wants them.
fn client(server: &MockServer) -> Client {
    Client::builder()
        .token("test-token")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 0,
            ..RetryPolicy::default()
        })
        .timeout(Duration::from_secs(5))
        .build()
        .expect("the client should build")
}

fn envelope(result: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "invocationInfo": { "req-id": "test" },
        "result": result,
    }))
}

#[tokio::test]
async fn every_request_carries_the_android_client_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .and(header("authorization", "OAuth test-token"))
        .and(header(
            "x-yandex-music-client",
            "YandexMusicAndroid/24023621",
        ))
        .and(header("accept-language", "ru"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 42 } })))
        .expect(1)
        .mount(&server)
        .await;

    let status = client(&server).account_status().await.unwrap();
    assert_eq!(status.uid(), Some(42));
}

#[tokio::test]
async fn init_caches_the_uid_so_later_calls_can_default_to_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 777 } })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users/777/likes/albums"))
        .respond_with(envelope(&serde_json::json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    // before init there is no uid to fall back to, and that is a clear error
    assert!(matches!(
        client.liked_albums(None).await,
        Err(Error::IdMissing(_))
    ));

    client.init().await.unwrap();
    assert_eq!(client.account_uid(), Some(777));
    assert!(client.liked_albums(None).await.unwrap().is_empty());
}

#[tokio::test]
async fn batch_lookups_post_their_ids_as_a_repeated_form_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/tracks"))
        .and(body_string_contains("track-ids=1"))
        .and(body_string_contains("track-ids=2"))
        .and(body_string_contains("with-positions=true"))
        .respond_with(envelope(&serde_json::json!([
            { "id": 1, "title": "first" },
            { "id": 2, "title": "second" },
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let tracks = client(&server).tracks([1, 2]).await.unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[1].title.as_deref(), Some("second"));
}

#[tokio::test]
async fn a_long_id_list_is_split_across_requests_instead_of_being_rejected() {
    let server = MockServer::start().await;
    // one request per chunk, each answering with the ids it was asked for
    Mock::given(method("POST"))
        .and(path("/tracks"))
        .respond_with(|request: &Request| {
            let body = String::from_utf8_lossy(&request.body).into_owned();
            let ids: Vec<serde_json::Value> = body
                .split('&')
                .filter_map(|pair| pair.strip_prefix("track-ids="))
                .map(|id| serde_json::json!({ "id": id.parse::<i64>().unwrap() }))
                .collect();
            envelope(&serde_json::Value::Array(ids))
        })
        .expect(3)
        .mount(&server)
        .await;

    let chunk = i64::try_from(yamuse::BATCH_CHUNK).unwrap();
    let ids: Vec<i64> = (0..=chunk * 2).collect();
    let tracks = client(&server).tracks(ids.clone()).await.unwrap();

    // every id came back, in order, as though it had been one request
    assert_eq!(tracks.len(), ids.len());
    assert_eq!(tracks[0].id.as_ref().unwrap().to_string(), "0");
    assert_eq!(
        tracks.last().unwrap().id.as_ref().unwrap().to_string(),
        ids.last().unwrap().to_string()
    );
}

#[tokio::test]
async fn a_short_id_list_still_takes_exactly_one_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/tracks"))
        .respond_with(envelope(&serde_json::json!([{ "id": 1 }])))
        .expect(1)
        .mount(&server)
        .await;

    assert_eq!(client(&server).tracks([1]).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_long_like_is_chunked_and_reports_the_last_revision() {
    let server = MockServer::start().await;
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&seen);
    Mock::given(method("POST"))
        .and(path("/users/1/likes/tracks/add-multiple"))
        .respond_with(move |_: &Request| {
            let nth = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            envelope(&serde_json::json!({ "revision": 100 + nth }))
        })
        .expect(2)
        .mount(&server)
        .await;

    let chunk = i64::try_from(yamuse::BATCH_CHUNK).unwrap();
    let ids: Vec<i64> = (0..=chunk).collect();
    let liked = client(&server)
        .set_liked(yamuse::Likeable::Track, ids, true, Some(1))
        .await
        .unwrap();

    // the last chunk's revision is the one describing the finished library
    assert_eq!(liked.revision, Some(101));
}

#[tokio::test]
async fn search_sends_every_parameter_the_api_expects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("text", "nine inch nails"))
        .and(query_param("type", "track"))
        .and(query_param("page", "2"))
        .and(query_param("nocorrect", "true"))
        .and(query_param("playlist-in-best", "true"))
        .respond_with(envelope(&serde_json::json!({
            "text": "nine inch nails",
            "tracks": { "total": 1, "perPage": 20, "results": [{ "id": 7 }] },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let query = yamuse::SearchQuery::default()
        .of_type(yamuse::SearchType::Track)
        .page(2)
        .exact();
    let results = client(&server)
        .search("nine inch nails", query)
        .await
        .unwrap();
    assert_eq!(results.track_results().len(), 1);
}

#[tokio::test]
async fn the_lossless_endpoint_signs_its_request_and_unwraps_the_payload() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/get-file-info"))
        .and(query_param("trackId", "42"))
        .and(query_param("quality", "lossless"))
        .and(query_param("transports", "encraw"))
        .respond_with(envelope(&serde_json::json!({
            "downloadInfo": {
                "trackId": "42",
                "codec": "flac",
                "transport": "encraw",
                "key": "00112233445566778899aabbccddeeff",
                "urls": ["https://cdn1.example/audio", "https://cdn2.example/audio"],
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let info = client(&server)
        .track_lossless_info(42, "lossless")
        .await
        .unwrap();
    assert!(info.is_lossless());
    assert!(info.is_encrypted());
    assert_eq!(info.links().len(), 2);

    // the signature and timestamp must both be present, or the api answers 400
    let request = &server.received_requests().await.unwrap()[0];
    let url = request.url.to_string();
    assert!(url.contains("sign="), "missing signature: {url}");
    assert!(url.contains("ts="), "missing timestamp: {url}");
}

#[tokio::test]
async fn a_compound_track_id_is_normalized_before_signing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/get-file-info"))
        .and(query_param("trackId", "42"))
        .respond_with(envelope(
            &serde_json::json!({ "downloadInfo": { "codec": "mp3" } }),
        ))
        .expect(1)
        .mount(&server)
        .await;

    // "42:1001" is a track-and-album pair; only the track id may be signed
    let info = client(&server)
        .track_lossless_info("42:1001", "lossless")
        .await
        .unwrap();
    assert!(!info.is_lossless());
}

#[tokio::test]
async fn statuses_map_onto_typed_errors() {
    for (status, matches) in [
        (401u16, "unauthorized"),
        (403, "unauthorized"),
        (400, "bad request"),
        (404, "not found"),
        (409, "conflict"),
        (413, "payload too large"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/account/status"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(serde_json::json!({ "name": "Err", "message": "nope" })),
            )
            .mount(&server)
            .await;

        let error = client(&server).account_status().await.unwrap_err();
        let rendered = error.to_string();
        assert!(
            rendered.starts_with(matches),
            "status {status} rendered as {rendered:?}, expected {matches:?}"
        );
        assert!(
            rendered.contains("Err: nope"),
            "the api message should survive: {rendered}"
        );
    }
}

#[tokio::test]
async fn idempotent_gets_retry_transient_failures() {
    let server = MockServer::start().await;
    // one 503, then success — a retrying client should never see the failure
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 5 } })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 2,
            base: Duration::from_millis(1),
            max: Duration::from_millis(2),
            ..RetryPolicy::default()
        })
        .build()
        .unwrap();

    assert_eq!(client.account_status().await.unwrap().uid(), Some(5));
}

#[tokio::test]
async fn mutations_are_never_retried() {
    let server = MockServer::start().await;
    // exactly one attempt, even though the policy allows retries: replaying a
    // mutation could double-apply it.
    Mock::given(method("POST"))
        .and(path("/users/1/likes/tracks/add-multiple"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 3,
            base: Duration::from_millis(1),
            max: Duration::from_millis(2),
            ..RetryPolicy::default()
        })
        .build()
        .unwrap();

    let outcome = client
        .set_liked(yamuse::Likeable::Track, [10], true, Some(1))
        .await;
    assert!(outcome.is_err());
}

#[tokio::test]
async fn deterministic_rejections_are_not_retried_either() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 3,
            base: Duration::from_millis(1),
            max: Duration::from_millis(2),
            ..RetryPolicy::default()
        })
        .build()
        .unwrap();

    assert!(matches!(
        client.account_status().await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn a_track_like_is_confirmed_by_the_revision_the_api_returns() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/1/likes/tracks/add-multiple"))
        .and(body_string_contains("track-ids=10"))
        .respond_with(envelope(&serde_json::json!({ "revision": 12 })))
        .expect(1)
        .mount(&server)
        .await;

    let liked = client(&server)
        .set_liked(yamuse::Likeable::Track, [10], true, Some(1))
        .await
        .unwrap();
    assert_eq!(
        liked.revision,
        Some(12),
        "the revision the api reports is handed back, not discarded"
    );
}

#[tokio::test]
async fn non_track_likes_are_confirmed_by_a_bare_ok() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/1/likes/albums/add-multiple"))
        .respond_with(envelope(&serde_json::json!("ok")))
        .expect(1)
        .mount(&server)
        .await;

    let liked = client(&server)
        .set_liked(yamuse::Likeable::Album, [5], true, Some(1))
        .await
        .unwrap();
    assert_eq!(liked.revision, None, "this endpoint reports no revision");
}

#[tokio::test]
async fn a_removal_targets_the_remove_path_not_add_multiple() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/1/likes/tracks/remove"))
        .respond_with(envelope(&serde_json::json!({ "revision": 13 })))
        .expect(1)
        .mount(&server)
        .await;

    let removed = client(&server)
        .set_liked(yamuse::Likeable::Track, [10], false, Some(1))
        .await
        .unwrap();
    assert_eq!(removed.revision, Some(13));
}

#[tokio::test]
async fn the_liked_library_is_unwrapped_from_its_wrapper() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/1/likes/tracks"))
        .respond_with(envelope(&serde_json::json!({
            "library": { "uid": 1, "revision": 9, "tracks": [{ "id": "10:20" }] }
        })))
        .mount(&server)
        .await;

    let library = client(&server).liked_tracks(Some(1)).await.unwrap();
    assert_eq!(library.revision, Some(9));
    assert_eq!(library.tracks.len(), 1);
}

#[tokio::test]
async fn queue_requests_carry_the_device_descriptor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/queues"))
        .and(header("x-yandex-music-device", "os=Custom; model=test"))
        .respond_with(envelope(&serde_json::json!({
            "queues": [{ "id": "q1", "context": { "type": "playlist" } }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .device("os=Custom; model=test")
        .build()
        .unwrap();

    let queues = client.queues(None).await.unwrap();
    assert_eq!(queues[0].id.as_deref(), Some("q1"));
}

#[tokio::test]
async fn a_queue_position_update_splits_its_parameters_across_query_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/queues/q1/update-position"))
        .and(query_param("currentIndex", "7"))
        .and(body_string_contains("isInteractive=false"))
        .and(header("x-yandex-music-device", "os=Custom; model=test"))
        .respond_with(envelope(&serde_json::json!("ok")))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .device("os=Custom; model=test")
        .build()
        .unwrap();

    client.update_queue_position("q1", 7, None).await.unwrap();
}

#[tokio::test]
async fn a_separator_in_a_path_parameter_cannot_retarget_the_request() {
    let server = MockServer::start().await;
    // the escaped form is what must be requested. wiremock matches `path` against
    // the decoded path, so this asserts the server saw one segment, not three.
    Mock::given(method("GET"))
        .and(path("/playlist/..%2F..%2Faccount%2Fstatus"))
        .respond_with(envelope(&serde_json::json!({ "kind": 1 })))
        .expect(1)
        .mount(&server)
        .await;
    // and that nothing reached the endpoint the traversal was aiming at
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let playlist = client(&server)
        .playlist_by_uuid("../../account/status")
        .await
        .unwrap();
    assert_eq!(playlist.kind, Some(1));
}

#[tokio::test]
async fn an_identifier_the_api_writes_with_a_colon_survives_escaping() {
    let server = MockServer::start().await;
    // `genre:pop` is the api's own spelling; escaping the colon would change what
    // the server routes on, so it has to reach the wire verbatim.
    Mock::given(method("GET"))
        .and(path("/rotor/station/genre:pop/tracks"))
        .respond_with(envelope(&serde_json::json!({ "batchId": "b1" })))
        .expect(1)
        .mount(&server)
        .await;

    let tracks = client(&server)
        .rotor_station_tracks("genre:pop", None)
        .await
        .unwrap();
    assert_eq!(tracks.batch_id.as_deref(), Some("b1"));
}

#[tokio::test]
async fn a_rate_limited_get_waits_the_delay_the_server_asked_for_and_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 5 } })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 1,
            // the ramp would fire almost immediately; the header must win over it
            base: Duration::from_millis(1),
            max: Duration::from_millis(1),
            retry_after_max: Duration::from_secs(30),
        })
        .build()
        .unwrap();

    let started = std::time::Instant::now();
    assert_eq!(client.account_status().await.unwrap().uid(), Some(5));
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "the retry should have waited the second the server asked for, waited {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_server_named_delay_is_capped_rather_than_honoured_verbatim() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "3600"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 5 } })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .retry(RetryPolicy {
            retries: 1,
            base: Duration::from_millis(1),
            max: Duration::from_millis(1),
            // an hour is what this api really does ask for; the cap is why a
            // watcher recovers instead of going quiet for the rest of the day
            retry_after_max: Duration::from_millis(50),
        })
        .build()
        .unwrap();

    let started = std::time::Instant::now();
    assert_eq!(client.account_status().await.unwrap().uid(), Some(5));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn a_rate_limit_that_outlives_the_retries_surfaces_with_its_delay() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "42")
                .set_body_json(serde_json::json!({ "message": "too many requests" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server).account_status().await.unwrap_err();
    assert!(
        matches!(&error, Error::RateLimited { retry_after, .. }
            if *retry_after == Some(Duration::from_secs(42))),
        "{error:?}"
    );
    assert!(
        error.is_retryable(),
        "a rate limit says not now, not not ever"
    );
    assert!(error.to_string().contains("too many requests"));
}

#[tokio::test]
async fn a_playlist_diff_is_posted_as_the_serialized_operation_list() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/1/playlists/1000/change"))
        .and(body_string_contains("revision=5"))
        .and(body_string_contains("insert"))
        .respond_with(envelope(
            &serde_json::json!({ "kind": 1000, "revision": 6 }),
        ))
        .expect(1)
        .mount(&server)
        .await;

    // the uid comes from init(), which is how a real caller reaches this endpoint
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(envelope(&serde_json::json!({ "account": { "uid": 1 } })))
        .mount(&server)
        .await;

    let client = client(&server);
    client.init().await.unwrap();

    let playlist = client
        .add_track_to_playlist(1000, 42, 1001, 0, 5)
        .await
        .unwrap();
    assert_eq!(playlist.revision, Some(6));
}

#[tokio::test]
async fn an_empty_body_deserializes_as_an_absent_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed/wizard/is-passed"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    // no body at all must not be an error; the flag simply reads false
    assert!(!client(&server).feed_wizard_is_passed().await.unwrap());
}

#[tokio::test]
async fn a_non_json_body_is_reported_as_an_invalid_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>maintenance</html>"))
        .mount(&server)
        .await;

    assert!(matches!(
        client(&server).account_status().await,
        Err(Error::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn a_timeout_is_reported_as_such_rather_than_a_generic_network_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/account/status"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .timeout(Duration::from_millis(50))
        .retry(RetryPolicy {
            retries: 0,
            ..RetryPolicy::default()
        })
        .build()
        .unwrap();

    assert!(matches!(
        client.account_status().await,
        Err(Error::TimedOut)
    ));
}

#[tokio::test]
async fn the_language_setting_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/genres"))
        .and(header("accept-language", "en"))
        .respond_with(envelope(
            &serde_json::json!([{ "id": "rock", "title": "Rock" }]),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .language(yamuse::Language::En)
        .build()
        .unwrap();

    let genres = client.genres().await.unwrap();
    assert_eq!(genres[0].localized_title("en"), Some("Rock"));
}

#[tokio::test]
async fn the_landing_endpoint_repeats_its_block_names_and_sends_the_sample_user() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/landing3"))
        .and(query_param("eitherUserId", "10254713668400548221"))
        .respond_with(envelope(&serde_json::json!({ "blocks": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let landing = client(&server)
        .landing(["personalplaylists", "chart"])
        .await
        .unwrap();
    assert!(landing.blocks.is_empty());

    let request: &Request = &server.received_requests().await.unwrap()[0];
    let url = request.url.to_string();
    assert!(url.contains("blocks=personalplaylists"), "{url}");
    assert!(url.contains("blocks=chart"), "{url}");
}

#[tokio::test]
async fn rotor_feedback_puts_the_batch_id_in_the_query_and_the_rest_in_the_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/rotor/station/user:onyourwave/feedback"))
        .and(query_param("batch-id", "batch-7"))
        .and(body_string_contains("type=trackFinished"))
        .and(body_string_contains("trackId=42"))
        .and(body_string_contains("totalPlayedSeconds=180"))
        .respond_with(envelope(&serde_json::json!("ok")))
        .expect(1)
        .mount(&server)
        .await;

    let report = yamuse::RotorFeedbackReport {
        track_id: Some(42),
        batch_id: Some("batch-7".into()),
        total_played_seconds: Some(180.0),
        ..Default::default()
    };
    client(&server)
        .rotor_feedback(
            "user:onyourwave",
            yamuse::RotorFeedback::TrackFinished,
            &report,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_mutation_the_api_does_not_acknowledge_is_an_error_not_a_quiet_false() {
    let server = MockServer::start().await;
    // a 200 whose body says the change did not happen. the old `Result<bool>`
    // reported this as `Ok(false)`, which no caller checked.
    Mock::given(method("POST"))
        .and(path("/users/1/likes/tracks/add-multiple"))
        .respond_with(envelope(&serde_json::json!({ "status": "error" })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .set_liked(yamuse::Likeable::Track, [10], true, Some(1))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidResponse(_)), "{error:?}");
    assert!(error.to_string().contains("did not acknowledge"), "{error}");
}

#[tokio::test]
async fn a_field_whose_type_changed_upstream_costs_that_field_not_the_request() {
    // this is the failure the search example hit against the live api: a field the
    // upstream types call a boolean arrived as an array, and the whole response
    // died with `invalid type: sequence, expected a boolean`.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(envelope(&serde_json::json!({
            "text": "query",
            "tracks": {
                "total": 2,
                "results": [
                    { "id": 1, "title": "first" },
                    { "id": 2, "title": "second", "best": ["not", "a", "boolean"] },
                ],
            },
        })))
        .mount(&server)
        .await;

    let dropped = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&dropped);
    let client = Client::builder()
        .token("t")
        .base_url(server.uri())
        .on_field_repair(move |field| sink.lock().unwrap().push(field.path.clone()))
        .build()
        .unwrap();

    let results = client
        .search("query", yamuse::SearchQuery::default())
        .await
        .unwrap();

    // the request succeeds and every track arrives
    assert_eq!(results.track_results().len(), 2);
    assert_eq!(results.track_results()[1].title.as_deref(), Some("second"));
    // only the mistyped field is gone
    assert_eq!(results.track_results()[1].best, None);
    // and the caller was told, rather than left guessing
    assert_eq!(
        dropped.lock().unwrap().as_slice(),
        ["tracks.results[1].best"]
    );
}

#[tokio::test]
async fn drift_is_tolerated_silently_when_no_hook_is_installed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/tracks"))
        .respond_with(envelope(&serde_json::json!([
            { "id": 1, "title": "t", "explicit": [] }
        ])))
        .mount(&server)
        .await;

    let tracks = client(&server).tracks([1]).await.unwrap();
    assert_eq!(tracks[0].title.as_deref(), Some("t"));
    assert_eq!(tracks[0].explicit, None);
}
