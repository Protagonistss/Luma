use std::fs;
use std::path::PathBuf;

use luma_lib::playlist::{download_playlist, parse_m3u, PlaylistSource};
use luma_lib::storage;

fn demo_playlist_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("samples")
        .join("demo-playlist.m3u")
}

fn init_temp_storage() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    storage::init(dir.path().to_path_buf()).expect("storage init");
    dir
}

#[test]
fn imports_demo_playlist_file() {
    let _guard = init_temp_storage();
    let content = fs::read_to_string(demo_playlist_path()).expect("read demo playlist");
    let playlist = storage::import_playlist_from_text(
        &content,
        PlaylistSource::from_file(
            &demo_playlist_path().to_string_lossy(),
            "demo-playlist.m3u",
        ),
    )
    .expect("import demo playlist");

    assert_eq!(playlist.channels.len(), 3);
    assert_eq!(playlist.channels[0].name, "Mux 测试直播");

    let channels = storage::list_channels(None).expect("list channels");
    assert_eq!(channels.len(), 3);

    let groups = storage::list_groups().expect("list groups");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "测试频道");
}

#[tokio::test]
async fn downloads_mux_test_stream_playlist() {
    let playlist = download_playlist("https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8")
        .await
        .expect("download mux playlist");

    assert_eq!(playlist.channels.len(), 1);
    assert!(playlist.channels[0]
        .stream_url
        .contains("test-streams.mux.dev"));
}

#[test]
fn parses_demo_playlist_without_storage() {
    let content = fs::read_to_string(demo_playlist_path()).expect("read demo playlist");
    let playlist = parse_m3u(&content).expect("parse demo playlist");
    assert_eq!(playlist.channels.len(), 3);
}
