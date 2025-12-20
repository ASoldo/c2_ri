use actix_web::{get, web, Error, HttpResponse};
use actix_web::http::header as actix_header;
use reqwest::header as reqwest_header;

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
