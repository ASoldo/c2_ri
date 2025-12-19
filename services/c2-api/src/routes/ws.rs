use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use c2_core::SecurityClassification;
use c2_identity::Permission;
use std::time::{Duration, Instant};

use crate::auth::authorize_request;
use crate::state::AppState;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct C2WsSession {
    last_heartbeat: Instant,
}

impl C2WsSession {
    pub fn new() -> Self {
        Self {
            last_heartbeat: Instant::now(),
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
            if Instant::now().duration_since(actor.last_heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping(b"ping");
        });
    }
}

impl Actor for C2WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_heartbeat(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for C2WsSession {
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
                ctx.text(format!("ack: {}", text));
            }
            Ok(ws::Message::Binary(bytes)) => {
                ctx.binary(bytes);
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                ctx.stop();
            }
        }
    }
}

#[get("/v1/stream/ws")]
pub async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    if let Err(response) = authorize_request(
        &req,
        &state.policy,
        Permission::ViewMissions,
        SecurityClassification::Unclassified,
    ) {
        return Ok(response);
    }

    ws::start(C2WsSession::new(), &req, stream)
}
