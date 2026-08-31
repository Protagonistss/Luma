use std::fs;
use luma_lib::playlist::parse_m3u;

/// Regression: a real-world aggregator list with http-user-agent attributes.
#[test]
fn parses_real_suxuang_sample_with_user_agent() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/suxuang-sample.m3u");
    let content = fs::read_to_string(path).expect("read fixture");
    let playlist = parse_m3u(&content).expect("parse");

    let with_ua = playlist
        .channels
        .iter()
        .filter(|c| c.user_agent.is_some())
        .count();
    let henan = playlist
        .channels
        .iter()
        .find(|c| c.name.contains("河南卫视"))
        .expect("henan channel exists");
    println!("channels: {}, with UA: {}", playlist.channels.len(), with_ua);
    println!("henan UA: {:?}", henan.user_agent);
    assert!(with_ua > 50, "expected dozens of UA-tagged channels");
    assert_eq!(henan.user_agent.as_deref(), Some("AptvPlayer-UA"));
}
