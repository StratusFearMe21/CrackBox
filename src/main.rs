use std::{
    net::SocketAddr,
    path::Path,
    sync::atomic::{AtomicU32, Ordering},
};

use axum::{
    extract::{self, ws::WebSocket, WebSocketUpgrade},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

static PC: AtomicU32 = AtomicU32::new(3);

#[derive(Deserialize)]
struct Config<'a> {
    #[serde(borrow)]
    tls: Option<Tls<'a>>,
    #[serde(borrow)]
    server: Server<'a>,
}

#[derive(Deserialize)]
struct Server<'a> {
    #[serde(borrow)]
    steam_apps_common: &'a Path,
    bind: SocketAddr,
}

#[derive(Deserialize)]
struct Tls<'a> {
    #[serde(borrow)]
    key: &'a Path,
    #[serde(borrow)]
    cert: &'a Path,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            if cfg!(debug_assertions) {
                std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| "crackbox=debug,tower_http=debug".into())
            } else {
                std::env::var("RUST_LOG").unwrap_or_default()
            },
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let file = std::fs::File::open("./config.toml").unwrap();
    let map = unsafe { memmap2::Mmap::map(&file).unwrap() };
    let config: Config = toml::from_slice(map.as_ref()).unwrap();

    let app = Router::new()
        .route("/api/v2/rooms", post(make_room))
        .route("/api/v2/rooms/:room_id/play", get(room_upgrade))
        .route("/api/v2/app-configs/:game_name", get(app_configs))
        .layer(TraceLayer::new_for_http());

    if let Some(tls) = config.tls {
        let rustls_config = RustlsConfig::from_pem_file(tls.cert, tls.key)
            .await
            .unwrap();

        tracing::debug!("HTTPS server started on {}", config.server.bind);
        axum_server::bind_rustls(config.server.bind, rustls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        tracing::debug!("HTTP server started on {}", config.server.bind);
        axum_server::bind(config.server.bind)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

#[derive(Serialize)]
struct RoomResponse {
    ok: bool,
    body: RoomResponseBody,
}

#[derive(Serialize)]
struct RoomResponseBody {
    host: &'static str,
    code: &'static str,
    token: &'static str,
}

async fn make_room() -> Json<RoomResponse> {
    Json(RoomResponse {
        ok: true,
        body: RoomResponseBody {
            host: "lbssexercise.info",
            code: "OKOK",
            token: "000000000000000000000000",
        },
    })
}

#[derive(Deserialize)]
struct RoomQuery {
    role: RoomRole,
    name: String,
    format: String,
    #[serde(rename = "user-id")]
    user_id: String,
}

#[derive(Deserialize)]
enum RoomRole {
    #[serde(rename = "player")]
    Player,
    #[serde(rename = "host")]
    Host,
}

async fn room_upgrade(
    extract::Path(room_id): extract::Path<String>,
    extract::Query(join_info): extract::Query<RoomQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    tracing::debug!("Upgrading room socket for room {}", room_id);
    match join_info.role {
        RoomRole::Host => ws.on_upgrade(host_handler),
        RoomRole::Player => ws.on_upgrade(player_handler),
    }
}

#[derive(Serialize)]
struct HostWelcome {
    pc: u32,
    opcode: &'static str,
    result: HostWelcomeResult,
}

#[derive(Serialize)]
struct HostWelcomeResult {
    id: u32,
    secret: &'static str,
    reconnect: bool,
    #[serde(rename = "deviceId")]
    device_id: &'static str,
    entities: (),
    here: (),
    profile: Option<HostWelcomeProfile>,
}

#[derive(Serialize)]
struct HostWelcomeProfile;

async fn host_handler(mut ws: WebSocket) {
    tracing::debug!("Sending host welcome");
    ws.send(axum::extract::ws::Message::Text(
        serde_json::to_string(&HostWelcome {
            pc: PC.fetch_add(1, Ordering::AcqRel),
            opcode: "client/welcome",
            result: HostWelcomeResult {
                id: 1,
                secret: "000000000000000000000000",
                reconnect: false,
                device_id: "0000000000.0000000000000000000000",
                entities: (),
                here: (),
                profile: None,
            },
        })
        .unwrap(),
    ))
    .await
    .unwrap();

    loop {
        ws.recv().await.unwrap().unwrap();
    }
}

async fn player_handler(mut ws: WebSocket) {
    tracing::debug!("Sending player welcome");
    ws.send(axum::extract::ws::Message::Text(
        serde_json::to_string(&HostWelcome {
            pc: PC.fetch_add(1, Ordering::AcqRel),
            opcode: "client/welcome",
            result: HostWelcomeResult {
                id: 1,
                secret: "000000000000000000000000",
                reconnect: false,
                device_id: "0000000000.0000000000000000000000",
                entities: (),
                here: (),
                profile: None,
            },
        })
        .unwrap(),
    ))
    .await
    .unwrap();

    loop {
        ws.recv().await.unwrap().unwrap();
    }
}

#[derive(Serialize)]
struct AppConfigs {
    ok: bool,
    body: AppConfigsBody,
}

#[derive(Serialize)]
struct AppConfigsBody {
    settings: AppConfigsSettings,
}

#[derive(Serialize)]
struct AppConfigsSettings {
    #[serde(rename = "serverUrl")]
    server_url: &'static str,
}

async fn app_configs(extract::Path(game_name): extract::Path<String>) -> Json<AppConfigs> {
    assert_eq!(game_name, "antique-freak");
    Json(AppConfigs {
        ok: true,
        body: AppConfigsBody {
            settings: AppConfigsSettings {
                server_url: "lbssexercise.info",
            },
        },
    })
}
