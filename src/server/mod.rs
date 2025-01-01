//! Local server, to host the project for development purposes.

use std::{future::IntoFuture, net::Ipv4Addr, path::PathBuf, time::Duration};

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
use color_eyre::eyre::Result;
use tokio::{net::TcpListener, sync::watch, time};
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
                    if rebuild.recv_async().await.is_err() {
                        break;
                    }
                    if tx.send(()).is_err() {
                        break;
                    }
                }
            });

            run_server(base, port, rx).await
        })
}

#[derive(Clone)]
struct AppState {
    shutdown: Shutdown,
    reload: watch::Receiver<()>,
}

async fn run_server(base: PathBuf, port: u16, notifier: watch::Receiver<()>) -> Result<()> {
    let index = ServeFile::new(base.join("dist/index.html"));
    let dist = ServeDir::new(base.join("dist"));
    let shutdown = Shutdown::new()?;

    let app = Router::new()
        .fallback_service(
            get_service(dist.fallback(index))
                .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .route("/__WAZZUP__/reload.js", get(reload_js))
        .route("/__WAZZUP__/reload", get(reload_ws))
        .with_state(AppState {
            shutdown: shutdown.clone(),
            reload: notifier,
        });

    // Always run on localhost only. It's a bad idea to publicly expose this server,
    // due to only doing the basics in terms of security.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
    let server = axum::serve(listener, app).into_future();

    tokio::select! {
        r = server => r?,
        () = shutdown.handle() => {}
    }

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
    State(AppState {
        shutdown,
        mut reload,
    }): State<AppState>,
) -> Response {
    reload.borrow_and_update();
    ws.on_upgrade(|socket| ws_notify(socket, shutdown, reload))
}

/// Notification logic, that listens for rebuilds on any components (triggered due to file changes)
/// and then notifies the frontend to reload.
async fn ws_notify(mut socket: WebSocket, shutdown: Shutdown, mut reload: watch::Receiver<()>) {
    loop {
        tokio::select! {
            () = shutdown.handle() => {
                return;
            }
            res = reload.changed() => {
                // reload channel closed
                if res.is_err() {
                    return;
                }

                let msg = Message::text("reload");

                // ensure we don't wait too long, so we don't miss out on any shutdown signal
                if time::timeout(Duration::from_secs(1), socket.send(msg)).await.is_err() {
                    continue;
                }
            }
            opt = socket.recv() => {
                // client closed the connection
                if opt.is_none() {
                    return;
                }
            }
        }
    }
}
