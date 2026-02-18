mod config;
mod injector;
mod payload;
mod remote;
mod requests;
mod response;

use crate::{
    injector::Injector,
    remote::{RemoteProcContainer, ScopedRemoteString},
    requests::MultiPayload,
    response::Response,
};
use axum::{
    Router,
    extract::Path,
    http::{StatusCode, Uri},
    response::Json,
    routing::{get, post},
    serve,
};
use dll_syringe::process::{OwnedProcess, Process};
use serde_json::json;
use std::{
    error::Error,
    net::SocketAddr,
    ops::Not,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, runtime::Builder, signal};
use tracing::{error, info, warn};

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .init();

    let options = config::Options::load()?;
    let target_name = options.target_name;
    let payload_path = options.payload_path;
    let port = options.port;
    let paths = options.paths;

    let procedure_table = payload::analyze_payload(&payload_path, paths)?;

    if let Some(target_process) = OwnedProcess::find_first_by_name(&target_name) {
        let pid = target_process.pid()?;
        let base_name = target_process
            .base_name()?
            .to_str()
            .unwrap_or("UNKNOWN BASE NAME")
            .to_string();
        let exec_path = target_process
            .path()?
            .to_str()
            .unwrap_or("UNKNOWN EXEC PATH")
            .to_string();

        info!(
            "injected process base name: {}, path: {}, pid: {}",
            base_name, exec_path, pid
        );

        if options.is_verbose
            && let Err(e) = payload::print_symbol_table(&procedure_table)
        {
            error!("failed to print symbol table: {:?}", e);
        }

        let mut injective = Injector::new(target_process, payload_path, procedure_table)?;

        let mut procedures = injective.regenerate();

        type Request = ((String, MultiPayload), mpsc::Sender<Result<String, String>>);
        let (cmd_tx, cmd_rx) = mpsc::channel::<Request>();

        let info = async move || {
            Json(json!({
                "base_name": base_name,
                "exec_path": exec_path,
                "pid": pid,
            }))
        };

        let thandle = thread::spawn(move || {
            let runtime = Builder::new_current_thread().enable_all().build().unwrap();
            let http_runtime_exit = runtime.block_on(async {
                let timeout = options.timeout;
                let app = Router::new()
                    .route("/info", get(info))
                    .route(
                        "/execute/{proc}",
                        post(
                            move |Path(proc): Path<String>, payload: MultiPayload| async move {
                                let start = Instant::now();
                                let (reply_tx, reply_rx) = mpsc::channel();

                                match cmd_tx
                                    .send(((proc, payload), reply_tx))
                                    .map_err(|e| e.to_string())
                                    .and_then(|_| {
                                        reply_rx
                                            .recv_timeout(Duration::from_millis(timeout))
                                            .map_err(|e| e.to_string())
                                    })
                                    .flatten()
                                {
                                    Ok(v) => {
                                        if v.starts_with("ERROR:") {
                                            error!("respons with error prefix: {}", v);
                                            (StatusCode::NOT_FOUND, Response::new(v, Some(&start)))
                                        } else {
                                            (StatusCode::OK, Response::new(v, Some(&start)))
                                        }
                                    }
                                    Err(e) => {
                                        error!("internal error: {}", e);
                                        (
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                            Response::new(e, Some(&start)),
                                        )
                                    }
                                }
                            },
                        ),
                    )
                    .fallback(async |uri: Uri| {
                        (
                            StatusCode::NOT_FOUND,
                            Response::new(format!("'{}' not found", uri), None),
                        )
                    });

                let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
                let listener = TcpListener::bind(addr).await?;

                serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await?;

                Ok::<_, Box<dyn Error>>(())
            });

            if let Err(e) = http_runtime_exit {
                error!("http runtime exit with error: {:?}", e);
            }
        });

        info!("REST procedure call available on http://localhost:{}", port);

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(options.timeout)) {
                Ok(((path, MultiPayload::Signal), reply_tx)) => {
                    if let Some(RemoteProcContainer::Signal(proc)) = procedures.get(&path) {
                        if let Err(e) = proc.call() {
                            reply_tx
                                .send(Err(e.to_string()))
                                .unwrap_or_else(channel_error_log);
                        } else {
                            reply_tx
                                .send(Ok("ACK".into()))
                                .unwrap_or_else(channel_error_log);
                        }
                    } else {
                        reply_tx
                            .send(Err("Invalid payload".into()))
                            .unwrap_or_else(channel_error_log);
                    }
                }
                Ok(((path, MultiPayload::Json(text)), reply_tx)) => {
                    if let Some(RemoteProcContainer::Text(proc)) = procedures.get(&path) {
                        let exchange =
                            ScopedRemoteString::new(injective.pid(), &text.payload.to_string())
                                .and_then(|s| proc.call(s.get_addr()).map_err(|e| e.into()))
                                .and_then(|u| ScopedRemoteString::from_remote(injective.pid(), u))
                                .and_then(|v| v.read_remote())
                                .map_err(|e| e.to_string())
                                .inspect_err(|e| error!("remote string write error: {}", e));

                        reply_tx.send(exchange).unwrap_or_else(channel_error_log);
                    } else {
                        reply_tx
                            .send(Err("Invalid payload".into()))
                            .unwrap_or_else(channel_error_log);
                    }
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if injective
                        .is_alive()
                        .inspect_err(|e| {
                            error!("health checking error: {}", e);
                            info!("killing process...");
                            if let Err(e) = injective.kill() {
                                error!("error killing process: {}", e);
                            }
                        })
                        .unwrap_or_default()
                        .not()
                    {
                        warn!("process is dead. reviving...");
                        match injective.renew().map(|_| injective.regenerate()) {
                            Ok(new_procedures) => {
                                procedures = new_procedures;
                                info!("process is revived")
                            }
                            Err(e) => error!("recovery error: {}", e),
                        }
                    }

                    if thandle.is_finished() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        if let Err(e) = thandle.join() {
            error!("axum thread closed with panic: {:?}", e);
        } else {
            info!("bye.");
        }
    } else {
        error!("'{}' is not running.", target_name);
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to set ctrl+c handler");
    };

    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("ctrl+c");
        },
        _ = terminate => {},
    }
}

fn channel_error_log<E>(e: E)
where
    E: std::fmt::Display,
{
    error!("channel error: {}", e);
}
