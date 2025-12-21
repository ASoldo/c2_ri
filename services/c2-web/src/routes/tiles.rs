use actix_web::{get, web, Error, HttpResponse};
use actix_web::http::header as actix_header;
use reqwest::header as reqwest_header;
use serde::Deserialize;

use crate::state::AppState;

#[get("/ui/tiles/{provider}/{z}/{x}/{y}")]
pub async fn tile(
    state: web::Data<AppState>,
    path: web::Path<(String, u8, u32, String)>,
) -> Result<HttpResponse, Error> {
    let (provider_id, z, x, y_raw) = path.into_inner();
    let provider = state
        .tile_providers
        .get(&provider_id)
        .ok_or_else(|| actix_web::error::ErrorNotFound("tile provider not found"))?;
    if z < provider.min_zoom || z > provider.max_zoom {
        return Ok(HttpResponse::BadRequest().body("zoom out of range"));
    }
    let y_digits: String = y_raw.chars().take_while(|c| c.is_ascii_digit()).collect();
    let y: u32 = y_digits
        .parse()
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid y"))?;

    let url = provider
        .url
        .replace("{z}", &z.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string());

    let response = state
        .tile_client
        .get(url)
        .header(
            "Accept",
            "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(actix_web::error::ErrorBadGateway)?;

    if !response.status().is_success() {
        return Ok(HttpResponse::BadGateway().finish());
    }
    let content_type = response
        .headers()
        .get(reqwest_header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response
        .bytes()
        .await
        .map_err(actix_web::error::ErrorBadGateway)?;
    let mut builder = HttpResponse::Ok();
    if let Some(content_type) = content_type.as_deref() {
        builder.insert_header((actix_header::CONTENT_TYPE, content_type));
    }
    Ok(builder
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Cross-Origin-Resource-Policy", "cross-origin"))
        .insert_header(("Cache-Control", "public, max-age=3600"))
        .body(bytes))
}

#[derive(Deserialize)]
pub struct WeatherQuery {
    field: Option<String>,
    time: Option<String>,
    format: Option<String>,
    gradient: Option<String>,
}

fn sanitize_time(value: &str) -> Option<String> {
    if value.len() > 40 {
        return None;
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | 'T' | 'Z' | '+' | '.'))
    {
        return None;
    }
    Some(value.to_string())
}

fn sanitize_format(value: &str) -> Option<String> {
    match value {
        "png" | "webp" | "jpg" | "jpeg" => Some(value.to_string()),
        _ => None,
    }
}

#[get("/ui/tiles/weather/{z}/{x}/{y}")]
pub async fn weather_tile(
    state: web::Data<AppState>,
    path: web::Path<(u8, u32, u32)>,
    query: web::Query<WeatherQuery>,
) -> Result<HttpResponse, Error> {
    let api_key = state
        .weather_api_key
        .as_ref()
        .ok_or_else(|| actix_web::error::ErrorNotFound("weather tiles disabled"))?;
    let (z, x, y) = path.into_inner();
    if z < state.weather_min_zoom || z > state.weather_max_zoom {
        return Ok(HttpResponse::BadRequest().body("zoom out of range"));
    }
    let field = query
        .field
        .as_deref()
        .unwrap_or(&state.weather_default_field);
    if !state.weather_fields.iter().any(|allowed| allowed == field) {
        return Ok(HttpResponse::BadRequest().body("field not allowed"));
    }
    let time = query
        .time
        .as_deref()
        .and_then(sanitize_time)
        .unwrap_or_else(|| state.weather_default_time.clone());
    let format = query
        .format
        .as_deref()
        .and_then(sanitize_format)
        .unwrap_or_else(|| state.weather_default_format.clone());
    let gradient = query
        .gradient
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| value.len() <= 200)
        .map(|value| value.to_string());

    let mut url = reqwest::Url::parse("https://api.tomorrow.io/v4/map/tile/")
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid weather tile url"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| actix_web::error::ErrorBadRequest("invalid weather tile url"))?;
        segments.push(&z.to_string());
        segments.push(&x.to_string());
        segments.push(&y.to_string());
        segments.push(field);
        segments.push(&format!("{}.{}", time, format));
    }
    url.query_pairs_mut().append_pair("apikey", api_key);
    if let Some(gradient) = gradient.as_deref() {
        url.query_pairs_mut().append_pair("gradient", gradient);
    }

    let response = state
        .tile_client
        .get(url)
        .header("Accept", "image/avif,image/webp,image/apng,image/*,*/*;q=0.8")
        .send()
        .await
        .map_err(actix_web::error::ErrorBadGateway)?;

    if !response.status().is_success() {
        return Ok(HttpResponse::BadGateway().finish());
    }
    let content_type = response
        .headers()
        .get(reqwest_header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response
        .bytes()
        .await
        .map_err(actix_web::error::ErrorBadGateway)?;
    let mut builder = HttpResponse::Ok();
    if let Some(content_type) = content_type.as_deref() {
        builder.insert_header((actix_header::CONTENT_TYPE, content_type));
    }
    Ok(builder
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Cross-Origin-Resource-Policy", "cross-origin"))
        .insert_header(("Cache-Control", "public, max-age=600"))
        .body(bytes))
}
