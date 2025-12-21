use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, StreamHandler};
use actix_web::{error::ErrorInternalServerError, get, web, Error, HttpRequest, HttpResponse};
use actix_web::web::Bytes;
use actix_web::rt::time::interval;
use actix_web_actors::ws;
use futures_util::stream::unfold;
use serde::Serialize;
use std::time::{Duration, Instant};

use crate::api::{ApiClient, UiEntitySnapshot, UiSnapshot};
use crate::render::{build_context, UiTemplateData};
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

#[get("/ui/entities")]
pub async fn entities(state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    if !state.api.auth_enabled() {
        return Ok(HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body("{\"error\":\"missing C2_UI_* auth configuration\"}"));
    }
    let payload = state
        .api
        .entities()
        .await
        .map_err(|err| ErrorInternalServerError(err.message))?;
    Ok(HttpResponse::Ok().json(payload))
}

#[get("/ui/stream/sse")]
pub async fn sse(state: web::Data<AppState>) -> HttpResponse {
    if !state.api.auth_enabled() {
        return HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body("{\"error\":\"missing C2_UI_* auth configuration\"}");
    }
    let api = state.api.clone();
    let tera = state.tera.clone();
    let service_name = state.config.service_name.clone();
    let environment = state.config.environment.to_string();
    let ticker = interval(api.poll_interval());
    let stream = unfold((ticker, api), move |(mut ticker, api)| {
        let tera = tera.clone();
        let service_name = service_name.clone();
        let environment = environment.clone();
        async move {
            ticker.tick().await;
            let entity_snapshot = api
                .entities()
                .await
                .unwrap_or_else(|_| UiEntitySnapshot::empty());
            let snapshot = UiSnapshot::from_entities(&entity_snapshot);
            let mut payload = String::new();
            match render_partials(&tera, &service_name, &environment, &snapshot) {
                Ok(partials) => {
                    payload.push_str(&build_sse_event(
                        "partials",
                        &PartialsPayload { fragments: partials },
                    ));
                }
                Err(err) => {
                    let error = StreamError { message: err };
                    payload.push_str(&build_sse_event("error", &error));
                }
            };
            payload.push_str(&build_sse_event("entities", &entity_snapshot));
            Some((Ok::<Bytes, actix_web::Error>(Bytes::from(payload)), (ticker, api)))
        }
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
    let session = UiWsSession::new(
        state.api.clone(),
        state.tera.clone(),
        state.config.service_name.clone(),
        state.config.environment.to_string(),
    );
    ws::start(session, &req, stream)
}

fn build_sse_event<T: Serialize>(event: &str, payload: &T) -> String {
    let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let mut output = String::new();
    output.push_str("event: ");
    output.push_str(event);
    output.push('\n');
    for line in data.lines() {
        output.push_str("data: ");
        output.push_str(line);
        output.push('\n');
    }
    output.push('\n');
    output
}

#[derive(Debug, Serialize)]
struct StreamError {
    message: String,
}

#[derive(Debug, Serialize)]
struct PartialFragment {
    target: &'static str,
    html: String,
}

#[derive(Debug, Serialize)]
struct PartialsPayload {
    fragments: Vec<PartialFragment>,
}

#[derive(Debug, Serialize)]
struct WsEnvelope<'a, T: Serialize> {
    kind: &'a str,
    payload: T,
}

struct UiWsSession {
    api: ApiClient,
    tera: tera::Tera,
    service_name: String,
    environment: String,
    last_heartbeat: Instant,
    poll_interval: Duration,
}

impl UiWsSession {
    fn new(api: ApiClient, tera: tera::Tera, service_name: String, environment: String) -> Self {
        let poll_interval = api.poll_interval();
        Self {
            api,
            tera,
            service_name,
            environment,
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
            let tera = actor.tera.clone();
            let service_name = actor.service_name.clone();
            let environment = actor.environment.clone();
            let fut = async move {
                api.entities()
                    .await
                    .unwrap_or_else(|_| UiEntitySnapshot::empty())
            };
            ctx.spawn(
                actix::fut::wrap_future(fut).map(
                    move |entity_snapshot, _actor, ctx: &mut ws::WebsocketContext<UiWsSession>| {
                        let snapshot = UiSnapshot::from_entities(&entity_snapshot);
                        match render_partials(&tera, &service_name, &environment, &snapshot) {
                            Ok(partials) => {
                                let envelope = WsEnvelope {
                                    kind: "partials",
                                    payload: PartialsPayload { fragments: partials },
                                };
                                if let Ok(text) = serde_json::to_string(&envelope) {
                                    ctx.text(text);
                                }
                                let envelope = WsEnvelope {
                                    kind: "entities",
                                    payload: entity_snapshot,
                                };
                                if let Ok(text) = serde_json::to_string(&envelope) {
                                    ctx.text(text);
                                }
                            }
                            Err(err) => {
                                let envelope = WsEnvelope {
                                    kind: "error",
                                    payload: StreamError { message: err },
                                };
                                if let Ok(text) = serde_json::to_string(&envelope) {
                                    ctx.text(text);
                                }
                            }
                        }
                    },
                ),
            );
        });
    }
}

fn render_partials(
    tera: &tera::Tera,
    service_name: &str,
    environment: &str,
    snapshot: &UiSnapshot,
) -> Result<Vec<PartialFragment>, String> {
    let data = UiTemplateData {
        service_name: service_name.to_string(),
        environment: environment.to_string(),
        status: None,
        snapshot: snapshot.clone(),
        tile_config_json: None,
        weather_config_json: None,
    };
    let context = build_context(&data);
    let mission_feed = tera
        .render("partials/mission_feed.html", &context)
        .map_err(|err| err.to_string())?;
    let incidents = tera
        .render("partials/incidents.html", &context)
        .map_err(|err| err.to_string())?;
    let assets = tera
        .render("partials/assets.html", &context)
        .map_err(|err| err.to_string())?;
    Ok(vec![
        PartialFragment {
            target: "mission-feed-panel",
            html: mission_feed,
        },
        PartialFragment {
            target: "incident-panel",
            html: incidents,
        },
        PartialFragment {
            target: "asset-panel",
            html: assets,
        },
    ])
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
