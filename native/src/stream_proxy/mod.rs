use std::time::Duration;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use url::Url;

const PROXY_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
pub struct StreamProxyState {
    pub base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct ProxyQuery {
    url: String,
}

pub async fn start_server() -> Result<StreamProxyState, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(PROXY_TIMEOUT_SECS))
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

pub fn wrap_stream_url(proxy_base: &str, stream_url: &str) -> Result<String, String> {
    validate_stream_url(stream_url)?;
    Ok(format!(
        "{}/proxy?url={}",
        proxy_base.trim_end_matches('/'),
        urlencoding::encode(stream_url)
    ))
}

#[tauri::command]
pub fn get_proxied_stream_url(
    state: tauri::State<'_, StreamProxyState>,
    stream_url: String,
) -> Result<String, String> {
    wrap_stream_url(&state.base_url, &stream_url)
}

async fn proxy_handler(
    State(state): State<StreamProxyState>,
    Query(query): Query<ProxyQuery>,
    headers: HeaderMap,
) -> Response {
    match proxy_request(&state, &query.url, &headers).await {
        Ok(response) => response,
        Err(message) => (StatusCode::BAD_GATEWAY, message).into_response(),
    }
}

async fn proxy_request(
    state: &StreamProxyState,
    target_url: &str,
    request_headers: &HeaderMap,
) -> Result<Response, String> {
    validate_stream_url(target_url)?;

    let mut request = state.client.get(target_url);
    if let Some(range) = request_headers.get(header::RANGE) {
        request = request.header(header::RANGE, range);
    }

    let response = request
        .send()
        .await
        .map_err(|err| format!("upstream request failed: {err}"))?;
    let status = response.status();
    let upstream_headers = response.headers().clone();
    let final_url = response.url().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("failed to read upstream body: {err}"))?;

    if !status.is_success() {
        return Err(format!("upstream returned http {status}"));
    }

    if is_m3u8_playlist(&final_url, &upstream_headers, &bytes) {
        let rewritten = rewrite_m3u8(&final_url, &state.base_url, &bytes);
        return Ok((
            StatusCode::OK,
            [
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/vnd.apple.mpegurl"),
                ),
            ],
            rewritten,
        )
            .into_response());
    }

    if looks_like_html(&bytes) {
        return Err("upstream returned a web page instead of a media stream".to_string());
    }

    let mut builder = Response::builder().status(status);
    builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

    if let Some(content_type) = upstream_headers.get(header::CONTENT_TYPE) {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    if let Some(content_range) = upstream_headers.get(header::CONTENT_RANGE) {
        builder = builder.header(header::CONTENT_RANGE, content_range);
    }
    if let Some(accept_ranges) = upstream_headers.get(header::ACCEPT_RANGES) {
        builder = builder.header(header::ACCEPT_RANGES, accept_ranges);
    }
    if let Some(content_length) = upstream_headers.get(header::CONTENT_LENGTH) {
        builder = builder.header(header::CONTENT_LENGTH, content_length);
    }

    builder
        .body(Body::from(bytes))
        .map_err(|err| err.to_string())
}

fn validate_stream_url(stream_url: &str) -> Result<(), String> {
    let parsed = Url::parse(stream_url).map_err(|err| err.to_string())?;
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        scheme => Err(format!("unsupported stream scheme: {scheme}")),
    }
}

fn is_m3u8_playlist(final_url: &Url, headers: &HeaderMap, body: &[u8]) -> bool {
    if body_starts_with_m3u(body) {
        return true;
    }

    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.contains("mpegurl")
                || value.contains("m3u8")
                || value.contains("application/vnd.apple.mpegurl")
        })
    {
        return body_starts_with_m3u(body);
    }

    final_url.path().ends_with(".m3u8") && body_starts_with_m3u(body)
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
    lowered.contains("<!doctype html")
        || lowered.contains("<html")
        || lowered.contains("<head")
}

fn rewrite_m3u8(base_url: &Url, proxy_base: &str, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    text.lines()
        .map(|line| rewrite_m3u8_line(base_url, proxy_base, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_m3u8_line(base_url: &Url, proxy_base: &str, line: &str) -> String {
    if let Some(rewritten) = rewrite_uri_attribute_line(base_url, proxy_base, line) {
        return rewritten;
    }

    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return line.to_string();
    }

    if trimmed.contains("/proxy?url=") {
        return line.to_string();
    }

    let absolute = resolve_url(base_url, trimmed);
    format!(
        "{}/proxy?url={}",
        proxy_base.trim_end_matches('/'),
        urlencoding::encode(&absolute)
    )
}

fn rewrite_uri_attribute_line(base_url: &Url, proxy_base: &str, line: &str) -> Option<String> {
    let uri_key = "URI=\"";
    let start = line.find(uri_key)?;
    let value_start = start + uri_key.len();
    let value_end = line[value_start..].find('"')? + value_start;
    let raw_uri = &line[value_start..value_end];
    let absolute = resolve_url(base_url, raw_uri);
    let proxied = format!(
        "{}/proxy?url={}",
        proxy_base.trim_end_matches('/'),
        urlencoding::encode(&absolute)
    );

    Some(format!(
        "{}{}{}",
        &line[..value_start],
        proxied,
        &line[value_end..]
    ))
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

    #[test]
    fn rewrites_segment_urls_in_playlist() {
        let base = Url::parse("http://74.91.26.218:82/live/cctv1hd.m3u8").expect("url");
        let body = "#EXTM3U\n#EXT-X-TARGETDURATION:10\nsegment-001.ts\n";
        let rewritten = rewrite_m3u8(&base, "http://127.0.0.1:17654", body.as_bytes());
        assert!(rewritten.contains("/proxy?url=http"));
        assert!(rewritten.contains("segment-001.ts"));
    }

    #[test]
    fn rewrites_uri_attribute() {
        let base = Url::parse("http://example.com/live/main.m3u8").expect("url");
        let line = r#"#EXT-X-KEY:METHOD=AES-128,URI="key.bin""#;
        let rewritten = rewrite_uri_attribute_line(&base, "http://127.0.0.1:17654", line)
            .expect("rewritten");
        assert!(rewritten.contains("/proxy?url=http"));
        assert!(rewritten.contains("key.bin"));
    }

    #[test]
    fn does_not_treat_html_as_playlist() {
        let url = Url::parse("http://example.com/live/cctv1.m3u8").expect("url");
        let body = b"<html><head><title>blocked</title></head></html>";
        assert!(!is_m3u8_playlist(&url, &HeaderMap::new(), body));
        assert!(looks_like_html(body));
    }

    #[test]
    fn rejects_unsupported_scheme() {
        assert!(validate_stream_url("rtmp://example.com/live").is_err());
    }
}
