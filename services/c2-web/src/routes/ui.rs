use actix_web::{error::ErrorInternalServerError, get, web, Error, HttpResponse};
use crate::state::AppState;
use crate::api::UiSnapshot;
use crate::render::{build_context, UiTemplateData};

#[get("/")]
pub async fn index(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let status = state.api.status().await.ok();
    let snapshot = state
        .api
        .snapshot()
        .await
        .unwrap_or_else(|_| UiSnapshot::empty());
    let data = UiTemplateData::from_state(&state, status, snapshot);
    let context = build_context(&data);
    let body = state
        .tera
        .render("index.html", &context)
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}
