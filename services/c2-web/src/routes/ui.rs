use actix_web::{error::ErrorInternalServerError, get, web, Error, HttpResponse};
use tera::Context;

use crate::state::AppState;

#[get("/")]
pub async fn index(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let mut context = Context::new();
    context.insert("service_name", &state.config.service_name);
    context.insert("environment", &state.config.environment.to_string());

    let body = state
        .tera
        .render("index.html", &context)
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}
