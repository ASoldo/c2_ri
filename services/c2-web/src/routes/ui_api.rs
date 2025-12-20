use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, StreamHandler};
use actix_web::{error::ErrorInternalServerError, get, web, Error, HttpRequest, HttpResponse};
use actix_web::web::Bytes;
use actix_web::rt::time::interval;
use actix_web_actors::ws;
use futures_util::stream::unfold;
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::api::ApiClient;
use crate::state::AppState;

#[get("/ui/status")]
pub async fn status(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let response = state
        .api
        .status()
        .await
        .map_err(|err| ErrorInternalServerError(err.message))?;
    Ok(HttpResponse::Ok().json(response))
}

#[get("/ui/summary")]
pub async fn summary(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    if !state.api.auth_enabled() {
        return Ok(HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body("{\"error\":\"missing C2_UI_* auth configuration\"}"));
    }
    let snapshot = state
        .api
        .snapshot()
        .await
        .map_err(|err| ErrorInternalServerError(err.message))?;
    Ok(HttpResponse::Ok().json(snapshot))
}

#[get("/ui/stream/sse")]
pub async fn sse(state: web::Data<AppState>) -> HttpResponse {
    if !state.api.auth_enabled() {
        return HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body("{\"error\":\"missing C2_UI_* auth configuration\"}");
    }
    let api = state.api.clone();
    let ticker = interval(api.poll_interval());
    let stream = unfold((ticker, api), |(mut ticker, api)| async move {
        ticker.tick().await;
        let payload = match api.snapshot().await {
            Ok(snapshot) => build_sse_event("snapshot", &snapshot),
            Err(err) => {
                let error = StreamError {
                    message: err.message,
                };
                build_sse_event("error", &error)
            }
        };
        Some((Ok::<Bytes, actix_web::Error>(Bytes::from(payload)), (ticker, api)))
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(stream)
}

#[get("/ui/stream/ws")]
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    if !state.api.auth_enabled() {
        return Ok(HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body("{\"error\":\"missing C2_UI_* auth configuration\"}"));
    }
    let session = UiWsSession::new(state.api.clone());
    ws::start(session, &req, stream)
}

fn build_sse_event<T: Serialize>(event: &str, payload: &T) -> String {
    let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    format!("event: {event}\ndata: {data}\n\n")
}

#[derive(Debug, Serialize)]
struct StreamError {
    message: String,
}

#[derive(Debug, Serialize)]
struct WsEnvelope<'a, T: Serialize> {
    kind: &'a str,
    payload: T,
}

struct UiWsSession {
    api: ApiClient,
    last_heartbeat: Instant,
    poll_interval: Duration,
}

impl UiWsSession {
    fn new(api: ApiClient) -> Self {
        let poll_interval = api.poll_interval();
        Self {
            api,
            last_heartbeat: Instant::now(),
            poll_interval,
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |actor, ctx| {
            if Instant::now().duration_since(actor.last_heartbeat) > Duration::from_secs(15) {
                ctx.stop();
                return;
            }
            ctx.ping(b"ping");
        });
    }

    fn start_updates(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(self.poll_interval, |actor, ctx| {
            let api = actor.api.clone();
            let fut = async move { api.snapshot().await };
            ctx.spawn(
                actix::fut::wrap_future(fut).map(
                    |result, _actor, ctx: &mut ws::WebsocketContext<UiWsSession>| match result {
                    Ok(snapshot) => {
                        let envelope = WsEnvelope {
                            kind: "snapshot",
                            payload: snapshot,
                        };
                        if let Ok(text) = serde_json::to_string(&envelope) {
                            ctx.text(text);
                        }
                    }
                    Err(err) => {
                        let envelope = WsEnvelope {
                            kind: "error",
                            payload: StreamError {
                                message: err.message,
                            },
                        };
                        if let Ok(text) = serde_json::to_string(&envelope) {
                            ctx.text(text);
                        }
                    }
                },
                ),
            );
        });
    }
}

impl Actor for UiWsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_heartbeat(ctx);
        self.start_updates(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for UiWsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(message)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&message);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                if text.trim().eq_ignore_ascii_case("ping") {
                    ctx.text("pong");
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}
