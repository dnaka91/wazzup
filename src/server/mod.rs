//! Local server, to host the project for development purposes.

use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, get_service},
    Router,
};
use tokio::sync::watch;
use tokio_shutdown::Shutdown;
use tower_http::services::{ServeDir, ServeFile};
use tracing::debug;

pub fn run(base: PathBuf, port: u16, rebuild: flume::Receiver<()>) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (tx, rx) = watch::channel(());
            tokio::spawn(async move {
                loop {
                    rebuild.recv_async().await.ok();
                    tx.send(()).ok();
                }
            });

            run_server(base, port, rx).await
        })
}

async fn run_server(base: PathBuf, port: u16, notifier: watch::Receiver<()>) -> Result<()> {
    let index = ServeFile::new(base.join("dist/index.html"));
    let dist = ServeDir::new(base.join("dist"));

    let app = Router::new()
        .fallback_service(
            get_service(dist.fallback(index))
                .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .route("/__WAZZUP__/reload.js", get(reload_js))
        .route("/__WAZZUP__/reload", get(reload_ws))
        .with_state(notifier);

    let shutdown = Shutdown::new()?;

    // Always run on localhost only. It's a bad idea to publicly expose this server,
    // due to only doing the basics in terms of security.
    axum::Server::try_bind(&(Ipv4Addr::LOCALHOST, port).into())?
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown.handle())
        .await?;

    debug!("server shut down");

    Ok(())
}

/// Provide the reload script that triggers page reloads on rebuild of components.
async fn reload_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript")],
        include_bytes!("reload.js"),
    )
}

/// Handle WebSocket connections from the reload script, triggering it to reload the page when
/// needed.
async fn reload_ws(
    ws: WebSocketUpgrade,
    State(mut notifier): State<watch::Receiver<()>>,
) -> Response {
    notifier.borrow_and_update();
    ws.on_upgrade(|socket| ws_notify(socket, notifier))
}

/// Notification logic, that listens for rebuilds on any components (triggered due to file changes)
/// and then notifies the frontend to reload.
async fn ws_notify(mut socket: WebSocket, mut notifier: watch::Receiver<()>) {
    loop {
        tokio::select! {
            res = notifier.changed() => {
                if res.is_err() {
                    return;
                }

                let msg = Message::Text("reload".to_owned());
                if socket.send(msg).await.is_err() {
                    return;
                }
            }
            opt = socket.recv() => {
                if opt.is_none() {
                    return;
                }
            }
        }
    }
}
