use axum::{Router, response::Redirect, routing::get};
use socketioxide::{SocketIo, extract::SocketRef};
use std::{net::SocketAddr, str::FromStr, sync::OnceLock};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use anyhow::anyhow;

use crate::{RUNTIME, models::packets::Packet};

const SERVER_ADDR: &str = "127.0.0.1:1305";

static SOCKET_IO: OnceLock<SocketIo> = OnceLock::new();

pub fn start_server() {
    RUNTIME.block_on(async {
        let (layer, io) = SocketIo::new_layer();
        io.ns("/", on_connect);
        if SOCKET_IO.set(io).is_err() {
            let e = anyhow!("Failed to initialize SocketIO");
            log::error!("{e}");
            panic!("{e}");
        }

        let app = Router::new().route("/", get(redirect_to_new_page)).layer(
            ServiceBuilder::new()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .layer(layer),
        );

        // HTTP
        axum_server::bind(SocketAddr::from_str(SERVER_ADDR).unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("{e}");
        }))
        .serve(app.into_make_service())
        .await
        .unwrap_or_else(|e| {
            log::error!("{e}");
            panic!("{e}");
        });
    });
}

async fn redirect_to_new_page() -> Redirect {
    Redirect::temporary("https://sranalysis.kain.id.vn")
}

fn on_connect(socket: SocketRef) {
    let packet = Packet::Connected {
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    socket.emit(&packet.name(), &packet.payload()).ok();
}

pub fn broadcast(packet: Packet) {
    RUNTIME.spawn(async move {
        if let Some(io) = SOCKET_IO.get() {
            io.broadcast()
                .emit(&packet.name(), &packet.payload())
                .await
                .unwrap_or_else(|e| {
                    log::error!("{e}");
                    panic!("{e}");
                });
        }
    });
}
