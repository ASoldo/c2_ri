use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web::http::header as actix_header;
use futures_util::{TryStreamExt, stream::try_unfold};
use reqwest::header as reqwest_header;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStdout, Command};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MediaQuery {
    url: String,
}

#[get("/ui/media-proxy")]
pub async fn media_proxy(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<MediaQuery>,
) -> Result<HttpResponse, Error> {
    let raw_url = query.url.trim();
    if raw_url.is_empty() {
        return Err(actix_web::error::ErrorBadRequest("missing url"));
    }
    let url = reqwest::Url::parse(raw_url)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid url"))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(actix_web::error::ErrorBadRequest(
                "unsupported url scheme",
            ))
        }
    }

    let mut request = state.tile_client.get(url.clone());
    if let Some(accept) = req
        .headers()
        .get(actix_header::ACCEPT)
        .and_then(|value| value.to_str().ok())
    {
        request = request.header(reqwest_header::ACCEPT, accept);
    }
    if let Some(range) = req
        .headers()
        .get(actix_header::RANGE)
        .and_then(|value| value.to_str().ok())
    {
        request = request.header(reqwest_header::RANGE, range);
    }

    let response = request
        .send()
        .await
        .map_err(actix_web::error::ErrorBadGateway)?;

    let status = actix_web::http::StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
    let mut builder = HttpResponse::build(status);
    if let Some(value) = response
        .headers()
        .get(reqwest_header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        builder.insert_header((actix_header::CONTENT_TYPE, value));
    }
    if let Some(value) = response
        .headers()
        .get(reqwest_header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
    {
        builder.insert_header((actix_header::CONTENT_LENGTH, value));
    }
    if let Some(value) = response
        .headers()
        .get(reqwest_header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
    {
        builder.insert_header((actix_header::CONTENT_RANGE, value));
    }
    if let Some(value) = response
        .headers()
        .get(reqwest_header::ACCEPT_RANGES)
        .and_then(|value| value.to_str().ok())
    {
        builder.insert_header((actix_header::ACCEPT_RANGES, value));
    }

    builder
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Cross-Origin-Resource-Policy", "cross-origin"))
        .insert_header(("Cache-Control", "no-store"));

    let content_type = response
        .headers()
        .get(reqwest_header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let is_playlist = content_type.contains("mpegurl")
        || url.path().to_ascii_lowercase().ends_with(".m3u8");

    if is_playlist {
        let body = response
            .bytes()
            .await
            .map_err(actix_web::error::ErrorBadGateway)?;
        let text = String::from_utf8_lossy(&body);
        let rewritten = rewrite_m3u8(&text, &url);
        return Ok(builder
            .insert_header((actix_header::CONTENT_TYPE, "application/vnd.apple.mpegurl"))
            .body(rewritten));
    }

    let stream = response
        .bytes_stream()
        .map_err(actix_web::error::ErrorBadGateway);
    Ok(builder.streaming(stream))
}

#[get("/ui/rtsp-proxy")]
pub async fn rtsp_proxy(
    _state: web::Data<AppState>,
    query: web::Query<MediaQuery>,
) -> Result<HttpResponse, Error> {
    let raw_url = query.url.trim();
    if raw_url.is_empty() {
        return Err(actix_web::error::ErrorBadRequest("missing url"));
    }
    let url = reqwest::Url::parse(raw_url)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid url"))?;
    if url.scheme() != "rtsp" {
        return Err(actix_web::error::ErrorBadRequest(
            "unsupported url scheme",
        ));
    }

    let mut child = Command::new("ffmpeg");
    child
        .arg("-loglevel")
        .arg("error")
        .arg("-rtsp_transport")
        .arg("tcp")
        .arg("-i")
        .arg(url.as_str())
        .arg("-an")
        .arg("-r")
        .arg("15")
        .arg("-f")
        .arg("mpjpeg")
        .arg("pipe:1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = child
        .spawn()
        .map_err(actix_web::error::ErrorBadGateway)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| actix_web::error::ErrorBadGateway("missing rtsp stdout"))?;
    let state = RtspStreamState { child, stdout };

    let stream = try_unfold(state, |mut state| async move {
        let mut buf = vec![0u8; 16 * 1024];
        let n = state
            .stdout
            .read(&mut buf)
            .await
            .map_err(actix_web::error::ErrorBadGateway)?;
        if n == 0 {
            return Ok::<_, actix_web::Error>(None);
        }
        buf.truncate(n);
        Ok(Some((web::Bytes::from(buf), state)))
    });

    Ok(HttpResponse::Ok()
        .insert_header((
            actix_header::CONTENT_TYPE,
            "multipart/x-mixed-replace; boundary=ffmpeg",
        ))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Cross-Origin-Resource-Policy", "cross-origin"))
        .insert_header(("Cache-Control", "no-store"))
        .streaming(stream))
}

struct RtspStreamState {
    child: Child,
    stdout: ChildStdout,
}

impl Drop for RtspStreamState {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn rewrite_m3u8(body: &str, base: &reqwest::Url) -> String {
    let mut lines = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('#') {
            lines.push(rewrite_uri_attributes(line, base));
            continue;
        }
        lines.push(rewrite_uri_line(trimmed, base));
    }
    let mut output = lines.join("\n");
    if body.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn rewrite_uri_line(line: &str, base: &reqwest::Url) -> String {
    match base.join(line) {
        Ok(joined) => proxy_url(&joined),
        Err(_) => line.to_string(),
    }
}

fn rewrite_uri_attributes(line: &str, base: &reqwest::Url) -> String {
    let mut output = String::with_capacity(line.len() + 32);
    let mut rest = line;
    while let Some(idx) = rest.find("URI=\"") {
        let (before, after) = rest.split_at(idx + 5);
        output.push_str(before);
        if let Some(end) = after.find('"') {
            let uri = &after[..end];
            let rewritten = match base.join(uri) {
                Ok(joined) => proxy_url(&joined),
                Err(_) => uri.to_string(),
            };
            output.push_str(&rewritten);
            rest = &after[end..];
        } else {
            output.push_str(after);
            return output;
        }
    }
    output.push_str(rest);
    output
}

fn proxy_url(target: &reqwest::Url) -> String {
    format!("/ui/media-proxy?url={}", encode_component(target.as_str()))
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(*byte as char),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}
