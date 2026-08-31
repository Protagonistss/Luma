use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::StreamExt;
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use url::Url;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 30;
/// Bytes buffered before deciding whether the response is a playlist that
/// needs rewriting or a media segment that can be streamed through as-is.
const SNIFF_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct StreamProxyState {
    base_url: String,
    /// Per-session secret appended to every proxied URL. Requests without a
    /// matching token are rejected so other pages in the user's browser (or
    /// other local processes) cannot use the proxy as an open relay.
    token: String,
    client: reqwest::Client,
}

impl StreamProxyState {
    /// Full signed proxy URL for `stream_url`, e.g.
    /// `http://127.0.0.1:43121/proxy?token=<token>&url=<encoded>`.
    pub fn wrap(&self, stream_url: &str) -> Result<String, String> {
        validate_stream_url(stream_url)?;
        Ok(signed_proxy_url(self, stream_url))
    }
}

#[derive(Debug, Deserialize)]
struct ProxyQuery {
    token: Option<String>,
    url: String,
}

pub async fn start_server() -> Result<StreamProxyState, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        // Per-read timeout instead of a total request timeout: live segments
        // can legitimately take longer than one total timeout to transfer.
        .read_timeout(Duration::from_secs(READ_TIMEOUT_SECS))
        .user_agent("Luma/0.1")
        .build()
        .map_err(|err| err.to_string())?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|err| err.to_string())?;
    let addr = listener.local_addr().map_err(|err| err.to_string())?;
    let base_url = format!("http://{addr}");

    let proxy_state = StreamProxyState {
        base_url: base_url.clone(),
        token: uuid::Uuid::new_v4().simple().to_string(),
        client: client.clone(),
    };

    let app = Router::new()
        .route("/proxy", get(proxy_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(proxy_state.clone());

    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("stream proxy server stopped: {err}");
        }
    });

    Ok(proxy_state)
}

#[tauri::command]
pub fn get_proxied_stream_url(
    state: tauri::State<'_, StreamProxyState>,
    stream_url: String,
) -> Result<String, String> {
    state.wrap(&stream_url)
}

async fn proxy_handler(
    State(state): State<StreamProxyState>,
    Query(query): Query<ProxyQuery>,
    headers: HeaderMap,
) -> Response {
    if query.token.as_deref() != Some(state.token.as_str()) {
        return (StatusCode::FORBIDDEN, "invalid proxy token").into_response();
    }

    match proxy_request(&state, &query.url, &headers).await {
        Ok(response) => response,
        Err(message) => (StatusCode::BAD_GATEWAY, message).into_response(),
    }
}

/// Request headers worth forwarding upstream. Many IPTV origins enforce
/// User-Agent / Referer checks and reject anonymous clients with 403.
const FORWARDED_REQUEST_HEADERS: &[HeaderName] = &[
    header::ACCEPT,
    header::ACCEPT_LANGUAGE,
    header::COOKIE,
    header::ORIGIN,
    header::REFERER,
    header::USER_AGENT,
];

/// Response headers copied from upstream. `content-length` is only set when
/// the body is streamed unchanged, so it still matches the real byte count.
const FORWARDED_RESPONSE_HEADERS: &[HeaderName] = &[
    header::CONTENT_TYPE,
    header::CONTENT_ENCODING,
    header::CONTENT_RANGE,
    header::ACCEPT_RANGES,
    header::CACHE_CONTROL,
];

async fn proxy_request(
    state: &StreamProxyState,
    target_url: &str,
    request_headers: &HeaderMap,
) -> Result<Response, String> {
    validate_stream_url(target_url)?;

    let mut request = state.client.get(target_url);
    for name in FORWARDED_REQUEST_HEADERS {
        if let Some(value) = request_headers.get(name) {
            request = request.header(name, value);
        }
    }
    if let Some(range) = request_headers.get(header::RANGE) {
        request = request.header(header::RANGE, range);
    }

    let mut response = request
        .send()
        .await
        .map_err(|err| format!("upstream request failed: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("upstream returned http {status}"));
    }

    let final_url = response.url().clone();
    let upstream_headers = response.headers().clone();

    // Buffer a prefix of the body to sniff its content type, then either
    // finish buffering (playlists are small and need rewriting) or stream
    // the rest straight through (media segments can be large).
    let mut prefix: Vec<Bytes> = Vec::new();
    let mut sniffed = 0usize;
    while sniffed < SNIFF_BYTES {
        match response
            .chunk()
            .await
            .map_err(|err| format!("failed to read upstream body: {err}"))?
        {
            Some(chunk) => {
                sniffed += chunk.len();
                prefix.push(chunk);
            }
            None => break,
        }
    }

    let sniffed_bytes: Vec<u8> = prefix.concat();

    if is_m3u8_playlist(&sniffed_bytes) {
        let rest = response
            .bytes()
            .await
            .map_err(|err| format!("failed to read upstream body: {err}"))?;
        let mut body = sniffed_bytes;
        body.extend_from_slice(&rest);

        let rewritten = rewrite_m3u8(&final_url, state, &body);
        return Ok((
            StatusCode::OK,
            [
                (
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                ),
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/vnd.apple.mpegurl"),
                ),
            ],
            rewritten,
        )
            .into_response());
    }

    if looks_like_html(&sniffed_bytes) {
        return Err("upstream returned a web page instead of a media stream".to_string());
    }

    let prefix_stream = futures_util::stream::iter(prefix.into_iter().map(Ok::<_, reqwest::Error>));
    let body_stream = prefix_stream.chain(response.bytes_stream());

    let mut builder = Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    for name in FORWARDED_RESPONSE_HEADERS {
        if let Some(value) = upstream_headers.get(name) {
            builder = builder.header(name, value);
        }
    }
    if let Some(content_length) = upstream_headers.get(header::CONTENT_LENGTH) {
        builder = builder.header(header::CONTENT_LENGTH, content_length);
    }

    builder
        .body(Body::from_stream(body_stream))
        .map_err(|err| err.to_string())
}

fn validate_stream_url(stream_url: &str) -> Result<(), String> {
    let parsed = Url::parse(stream_url).map_err(|err| err.to_string())?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("unsupported stream scheme: {scheme}")),
    }
}

fn is_m3u8_playlist(body: &[u8]) -> bool {
    body_starts_with_m3u(body)
}

fn body_starts_with_m3u(body: &[u8]) -> bool {
    let sample = &body[..body.len().min(512)];
    std::str::from_utf8(sample)
        .map(|text| text.contains("#EXTM3U"))
        .unwrap_or(false)
}

fn looks_like_html(body: &[u8]) -> bool {
    let sample = &body[..body.len().min(512)];
    let Ok(text) = std::str::from_utf8(sample) else {
        return false;
    };
    let lowered = text.to_ascii_lowercase();
    lowered.contains("<!doctype html") || lowered.contains("<html") || lowered.contains("<head")
}

fn rewrite_m3u8(base_url: &Url, state: &StreamProxyState, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    text.lines()
        .map(|line| rewrite_m3u8_line(base_url, state, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_m3u8_line(base_url: &Url, state: &StreamProxyState, line: &str) -> String {
    if let Some(rewritten) = rewrite_uri_attribute_line(base_url, state, line) {
        return rewritten;
    }

    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return line.to_string();
    }

    if trimmed.contains("/proxy?token=") {
        return line.to_string();
    }

    signed_proxy_url(state, &resolve_url(base_url, trimmed))
}

fn rewrite_uri_attribute_line(
    base_url: &Url,
    state: &StreamProxyState,
    line: &str,
) -> Option<String> {
    let uri_key = "URI=\"";
    let start = line.find(uri_key)?;
    let value_start = start + uri_key.len();
    let value_end = line[value_start..].find('"')? + value_start;
    let raw_uri = &line[value_start..value_end];
    let absolute = resolve_url(base_url, raw_uri);
    let proxied = signed_proxy_url(state, &absolute);

    Some(format!(
        "{}{}{}",
        &line[..value_start],
        proxied,
        &line[value_end..]
    ))
}

fn signed_proxy_url(state: &StreamProxyState, target: &str) -> String {
    format!(
        "{}/proxy?token={}&url={}",
        state.base_url.trim_end_matches('/'),
        state.token,
        urlencoding::encode(target)
    )
}

fn resolve_url(base_url: &Url, reference: &str) -> String {
    match base_url.join(reference) {
        Ok(url) => url.to_string(),
        Err(_) => reference.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt; // for `oneshot`

    fn test_state() -> StreamProxyState {
        StreamProxyState {
            base_url: "http://127.0.0.1:17654".to_string(),
            token: "test-token".to_string(),
            client: reqwest::Client::new(),
        }
    }

    #[test]
    fn rewrites_segment_urls_in_playlist() {
        let state = test_state();
        let base = Url::parse("http://74.91.26.218:82/live/cctv1hd.m3u8").expect("url");
        let body = "#EXTM3U\n#EXT-X-TARGETDURATION:10\nsegment-001.ts\n";
        let rewritten = rewrite_m3u8(&base, &state, body.as_bytes());
        assert!(rewritten.contains("/proxy?token=test-token&url=http"));
        assert!(rewritten.contains("segment-001.ts"));
    }

    #[test]
    fn rewrites_uri_attribute() {
        let state = test_state();
        let base = Url::parse("http://example.com/live/main.m3u8").expect("url");
        let line = r#"#EXT-X-KEY:METHOD=AES-128,URI="key.bin""#;
        let rewritten = rewrite_uri_attribute_line(&base, &state, line).expect("rewritten");
        assert!(rewritten.contains("/proxy?token=test-token&url=http"));
        assert!(rewritten.contains("key.bin"));
    }

    #[test]
    fn does_not_rewrite_already_proxied_lines() {
        let state = test_state();
        let base = Url::parse("http://example.com/live/main.m3u8").expect("url");
        let line =
            "http://127.0.0.1:17654/proxy?token=test-token&url=http%3A%2F%2Fexample.com%2Fseg.ts";
        assert_eq!(rewrite_m3u8_line(&base, &state, line), line);
    }

    #[test]
    fn does_not_treat_html_as_playlist() {
        let body = b"<html><head><title>blocked</title></head></html>";
        assert!(!is_m3u8_playlist(body));
        assert!(looks_like_html(body));
    }

    #[test]
    fn rejects_unsupported_scheme() {
        assert!(validate_stream_url("rtmp://example.com/live").is_err());
    }

    #[test]
    fn wrap_includes_token() {
        let state = test_state();
        let wrapped = state.wrap("http://example.com/live.m3u8").expect("wrapped");
        assert!(wrapped.starts_with("http://127.0.0.1:17654/proxy?token=test-token&url="));
    }

    #[tokio::test]
    async fn rejects_missing_or_wrong_token() {
        let state = test_state();
        let app = Router::new()
            .route("/proxy", get(proxy_handler))
            .with_state(state);

        let request = Request::builder()
            .uri("/proxy?url=http%3A%2F%2Fexample.com%2Fstream.ts")
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let request = Request::builder()
            .uri("/proxy?token=wrong&url=http%3A%2F%2Fexample.com%2Fstream.ts")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
