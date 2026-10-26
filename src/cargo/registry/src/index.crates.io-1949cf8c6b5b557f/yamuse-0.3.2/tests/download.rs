//! streaming downloads against a mock cdn.
//!
//! the point of these is the parts that only a real transfer exercises: that the
//! encraw keystream is applied while streaming (not after), that a failing mirror
//! falls through to the next, and that a failed transfer leaves no partial file.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use yamuse::download::{Downloader, Strategy};
use yamuse::{Client, crypto::decrypt_encraw};

const KEY: &str = "000102030405060708090a0b0c0d0e0f";

fn transport_client(server: &MockServer) -> Client {
    Client::builder()
        .token("t")
        .base_url(server.uri())
        .build()
        .expect("the client should build")
}

/// a payload large enough to arrive in several chunks.
fn payload() -> Vec<u8> {
    (0..=255u8).cycle().take(300_000).collect()
}

#[tokio::test]
async fn a_plain_stream_lands_on_disk_byte_for_byte() {
    let server = MockServer::start().await;
    let body = payload();
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.mp3");
    let client = transport_client(&server);

    let written = Downloader::new(client.transport(), [format!("{}/audio", server.uri())])
        .to_file(&file)
        .await
        .unwrap();

    assert_eq!(usize::try_from(written).unwrap(), body.len());
    assert_eq!(tokio::fs::read(&file).await.unwrap(), body);
}

#[tokio::test]
async fn an_encrypted_stream_is_decrypted_while_it_is_written() {
    let server = MockServer::start().await;
    let plain = payload();
    // the cdn serves ciphertext; ctr is symmetric, so encrypting is the same call
    let encrypted = decrypt_encraw(&plain, KEY).unwrap();
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(encrypted))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.flac");
    let client = transport_client(&server);

    Downloader::new(client.transport(), [format!("{}/audio", server.uri())])
        .encraw_key(KEY)
        .to_file(&file)
        .await
        .unwrap();

    // chunk-by-chunk decryption must reproduce the original exactly: a keystream
    // desynchronised at a chunk boundary would corrupt everything after it.
    assert_eq!(tokio::fs::read(&file).await.unwrap(), plain);
}

#[tokio::test]
async fn to_bytes_decrypts_too() {
    let server = MockServer::start().await;
    let plain = b"fLaC short payload".to_vec();
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(decrypt_encraw(&plain, KEY).unwrap()),
        )
        .mount(&server)
        .await;

    let client = transport_client(&server);
    let bytes = Downloader::new(client.transport(), [format!("{}/audio", server.uri())])
        .maybe_encraw_key(Some(KEY))
        .to_bytes()
        .await
        .unwrap();
    assert_eq!(bytes, plain);
}

#[tokio::test]
async fn a_dead_mirror_falls_through_to_the_next() {
    let server = MockServer::start().await;
    let body = payload();
    Mock::given(method("GET"))
        .and(path("/dead"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/live"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.mp3");
    let client = transport_client(&server);

    let urls = [
        format!("{}/dead", server.uri()),
        format!("{}/live", server.uri()),
    ];
    let written = Downloader::new(client.transport(), urls)
        .to_file(&file)
        .await
        .unwrap();

    assert_eq!(usize::try_from(written).unwrap(), body.len());
}

#[tokio::test]
async fn racing_mirrors_still_produces_the_right_bytes() {
    let server = MockServer::start().await;
    let body = payload();
    for name in ["a", "b", "c"] {
        Mock::given(method("GET"))
            .and(path(format!("/{name}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .mount(&server)
            .await;
    }

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.mp3");
    let client = transport_client(&server);

    let urls: Vec<String> = ["a", "b", "c"]
        .iter()
        .map(|n| format!("{}/{n}", server.uri()))
        .collect();
    Downloader::new(client.transport(), urls)
        .strategy(Strategy::Race)
        .to_file(&file)
        .await
        .unwrap();

    assert_eq!(tokio::fs::read(&file).await.unwrap(), body);
}

#[tokio::test]
async fn every_mirror_failing_is_an_error_and_leaves_no_partial_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/dead"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.mp3");
    let client = transport_client(&server);

    let urls = [
        format!("{}/dead", server.uri()),
        format!("{}/dead", server.uri()),
    ];
    assert!(
        Downloader::new(client.transport(), urls)
            .to_file(&file)
            .await
            .is_err()
    );
    // a truncated file left behind would look like a valid cached track
    assert!(
        !file.exists(),
        "a failed download must not leave a partial file"
    );
    assert!(
        !partial_of(&file).exists(),
        "nor may the scratch file outlive the attempt"
    );
}

/// where a download in flight parks its bytes.
fn partial_of(path: &std::path::Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    std::path::PathBuf::from(name)
}

#[tokio::test]
async fn a_failed_download_leaves_a_file_already_at_the_destination_untouched() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/dead"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.flac");
    // a track downloaded earlier, which a retry must not destroy
    let existing = b"the copy that was already there".to_vec();
    tokio::fs::write(&file, &existing).await.unwrap();

    let client = transport_client(&server);
    assert!(
        Downloader::new(client.transport(), [format!("{}/dead", server.uri())])
            .to_file(&file)
            .await
            .is_err()
    );

    assert_eq!(
        tokio::fs::read(&file).await.unwrap(),
        existing,
        "a failed download must not truncate what was already on disk"
    );
    assert!(!partial_of(&file).exists());
}

#[tokio::test]
async fn a_successful_download_replaces_the_destination_and_cleans_up_after_itself() {
    let server = MockServer::start().await;
    let body = payload();
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.flac");
    tokio::fs::write(&file, b"stale").await.unwrap();

    let client = transport_client(&server);
    Downloader::new(client.transport(), [format!("{}/audio", server.uri())])
        .to_file(&file)
        .await
        .unwrap();

    assert_eq!(tokio::fs::read(&file).await.unwrap(), body);
    assert!(
        !partial_of(&file).exists(),
        "the scratch file is renamed into place, not left beside it"
    );
}

#[tokio::test]
async fn a_transfer_that_stops_short_commits_nothing() {
    // wiremock cannot serve this: hyper refuses to send a body shorter than the
    // content-length it announced. a raw socket can, and a cdn dropping a
    // connection mid-track is exactly the case that matters — left unchecked the
    // short file lands under the real name, indistinguishable from a whole track.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        // read past the request line so the client is not blocked writing
        let mut scratch = [0u8; 1024];
        let _ = socket.read(&mut scratch).await;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\n\r\n")
            .await
            .unwrap();
        socket.write_all(&[0u8; 1024]).await.unwrap();
        // and then simply go away, mid-body
        drop(socket);
    });

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.flac");
    let client = Client::builder().token("t").build().unwrap();

    let error = Downloader::new(client.transport(), [format!("http://{addr}/truncated")])
        .to_file(&file)
        .await
        .unwrap_err();

    assert!(
        !file.exists(),
        "a short download must not be committed: {error}"
    );
    assert!(!partial_of(&file).exists());
}

#[tokio::test]
async fn racing_into_memory_produces_the_same_bytes_as_failing_over() {
    let server = MockServer::start().await;
    let body = payload();
    Mock::given(method("GET"))
        .and(path("/dead"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;

    let client = transport_client(&server);
    let urls = [
        format!("{}/dead", server.uri()),
        format!("{}/audio", server.uri()),
    ];

    let raced = Downloader::new(client.transport(), urls.clone())
        .strategy(Strategy::Race)
        .to_bytes()
        .await
        .unwrap();
    assert_eq!(raced, body);
}

#[tokio::test]
async fn a_download_with_no_urls_fails_fast() {
    let server = MockServer::start().await;
    let client = transport_client(&server);
    let dir = tempfile::tempdir().unwrap();

    let empty: [String; 0] = [];
    let downloader = Downloader::new(client.transport(), empty);
    assert!(downloader.to_file(dir.path().join("x.mp3")).await.is_err());
    assert!(downloader.to_bytes().await.is_err());
}

#[tokio::test]
async fn a_malformed_key_is_rejected_before_any_bytes_are_written() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 16]))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("track.flac");
    let client = transport_client(&server);

    let result = Downloader::new(client.transport(), [format!("{}/audio", server.uri())])
        .encraw_key("not-hex")
        .to_file(&file)
        .await;
    assert!(result.is_err());
    assert!(!file.exists());
}
