use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web::http::header as actix_header;
use futures_util::TryStreamExt;
use reqwest::header as reqwest_header;
use serde::Deserialize;

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

    let mut request = state.tile_client.get(url);
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

    let stream = response
        .bytes_stream()
        .map_err(actix_web::error::ErrorBadGateway);
    Ok(builder.streaming(stream))
}
