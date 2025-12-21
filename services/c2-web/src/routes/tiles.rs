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
        let status = actix_web::http::StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
        return Ok(HttpResponse::build(status).finish());
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
}

fn normalize_time(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lowered = value.to_ascii_lowercase();
    if lowered == "default" || lowered == "auto" || lowered == "latest" || lowered == "now" {
        return None;
    }
    if value.len() == 10
        && value.bytes().enumerate().all(|(idx, b)| {
            if idx == 4 || idx == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return Some(value.to_string());
    }
    None
}

fn sanitize_format(value: &str) -> Option<String> {
    match value {
        "png" | "jpg" | "jpeg" => Some(value.to_string()),
        _ => None,
    }
}

#[get("/ui/tiles/weather/{z}/{x}/{y}")]
pub async fn weather_tile(
    state: web::Data<AppState>,
    path: web::Path<(u8, u32, u32)>,
    query: web::Query<WeatherQuery>,
) -> Result<HttpResponse, Error> {
    if !state.weather_enabled {
        return Err(actix_web::error::ErrorNotFound("weather tiles disabled"));
    }
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
        .and_then(normalize_time)
        .or_else(|| normalize_time(&state.weather_default_time));
    let format = query
        .format
        .as_deref()
        .and_then(sanitize_format)
        .unwrap_or_else(|| state.weather_default_format.clone());

    let base = if let Some(time) = time.as_deref() {
        format!(
            "{}/{}/default/{}/{}/{}/{}/{}.{}",
            state.weather_base_url,
            field,
            time,
            state.weather_tile_matrix_set,
            z,
            y,
            x,
            format
        )
    } else {
        format!(
            "{}/{}/default/{}/{}/{}/{}.{}",
            state.weather_base_url,
            field,
            state.weather_tile_matrix_set,
            z,
            y,
            x,
            format
        )
    };
    let url = reqwest::Url::parse(&base)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid weather tile url"))?;
    let log_url = url.clone();
    let response = state
        .tile_client
        .get(url)
        .header("Accept", "image/avif,image/webp,image/apng,image/*,*/*;q=0.8")
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!(
                error = %err,
                url = %log_url,
                field = field,
                "weather tile request failed"
            );
            return Ok(HttpResponse::BadGateway().finish());
        }
    };

    if !response.status().is_success() {
        tracing::warn!(
            status = %response.status(),
            url = %log_url,
            field = field,
            "weather tile upstream error"
        );
        let status = actix_web::http::StatusCode::from_u16(response.status().as_u16())
            .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
        return Ok(HttpResponse::build(status).finish());
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
