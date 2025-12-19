use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_web::rt::time::interval;
use actix_web::web::Bytes;
use c2_core::SecurityClassification;
use c2_identity::Permission;
use futures_util::stream::unfold;
use std::time::Duration;

use crate::auth::authorize_request;
use crate::state::AppState;

#[get("/v1/stream/sse")]
pub async fn sse(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewMissions,
        SecurityClassification::Unclassified,
    ) {
        return response;
    }

    let interval = interval(Duration::from_secs(2));
    let stream = unfold((interval, 0u64), |(mut interval, counter)| async move {
        interval.tick().await;
        let payload = format!("event: heartbeat\ndata: {}\n\n", counter);
        let bytes = Bytes::from(payload);
        Some((Ok::<Bytes, actix_web::Error>(bytes), (interval, counter + 1)))
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
}
