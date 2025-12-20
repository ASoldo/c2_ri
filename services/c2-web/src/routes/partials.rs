use actix_web::{error::ErrorInternalServerError, get, web, Error, HttpResponse};

use crate::api::UiSnapshot;
use crate::render::{build_context, UiTemplateData};
use crate::state::AppState;

fn render_partial(
    state: &AppState,
    template: &str,
    snapshot: UiSnapshot,
) -> Result<String, tera::Error> {
    let data = UiTemplateData::from_state(state, None, snapshot);
    let context = build_context(&data);
    state.tera.render(template, &context)
}

#[get("/partials/mission-feed")]
pub async fn mission_feed(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let snapshot = state
        .api
        .snapshot()
        .await
        .unwrap_or_else(|_| UiSnapshot::empty());
    let body = render_partial(&state, "partials/mission_feed.html", snapshot)
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body))
}

#[get("/partials/incidents")]
pub async fn incidents(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let snapshot = state
        .api
        .snapshot()
        .await
        .unwrap_or_else(|_| UiSnapshot::empty());
    let body = render_partial(&state, "partials/incidents.html", snapshot)
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body))
}

#[get("/partials/assets")]
pub async fn assets(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let snapshot = state
        .api
        .snapshot()
        .await
        .unwrap_or_else(|_| UiSnapshot::empty());
    let body = render_partial(&state, "partials/assets.html", snapshot)
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body))
}
