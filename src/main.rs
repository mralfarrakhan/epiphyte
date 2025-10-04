use std::{
    collections::HashMap, error::Error, net::SocketAddr, sync::mpsc, thread, time::Duration,
};

use axum::{
    Router,
    extract::Path,
    http::{StatusCode, Uri},
    response::Json,
    routing::get,
    serve,
};
use dll_syringe::{
    Syringe,
    process::{OwnedProcess, Process},
};
use serde_json::json;
use tokio::{net::TcpListener, runtime::Builder, signal};

mod config;
mod payload;

fn main() -> Result<(), Box<dyn Error>> {
    let options = config::Options::load()?;
    let target_name = options.target_name;
    let payload_path = options.payload_path;
    let port = options.port;
    let paths = options.paths;

    let procedures = payload::analyze_payload(&payload_path, paths)?;

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

        println!(
            "[INFO] injected process base name: {}, path: {}, pid: {}.",
            base_name, exec_path, pid
        );

        if options.is_verbose {
            println!();
            if let Err(e) = payload::print_symbol_table(&procedures) {
                eprintln!("[ERROR] failed to print symbols table: {}", e);
            }
            println!();
        }

        let syringe = Syringe::for_process(target_process);
        let injected_payload = syringe.inject(payload_path)?;

        dbg!(&procedures);

        let procedures: HashMap<_, _> = procedures
            .into_iter()
            .filter_map(|(s, m)| {
                if s != "DllMain" && m.is_valid() {
                    let procedure = unsafe {
                        syringe.get_raw_procedure::<extern "system" fn()>(injected_payload, &s)
                    }
                    .ok()??;

                    Some((s, procedure))
                } else {
                    None
                }
            })
            .collect();

        dbg!(&procedures);

        println!(
            "[INFO] REST procedure call available on http://localhost:{}/",
            port,
        );

        let (cmd_tx, cmd_rx) = mpsc::channel();

        let info = async move || {
            Json(json!({
                "base_name": base_name,
                "exec_path": exec_path,
                "pid": pid,
            }))
        };

        let fallback = async |uri: Uri| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "message": format!("'{uri}' not found")
                })),
            )
        };

        let thandle = thread::spawn(move || {
            let runtime = Builder::new_current_thread().enable_all().build().unwrap();

            runtime.block_on(async {
                let app = Router::new()
                    .route("/info", get(info))
                    .route(
                        "/execute/{proc}",
                        get(|Path(proc): Path<String>| async move {
                            cmd_tx.send(proc).unwrap();
                        }),
                    )
                    .fallback(fallback);

                let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
                let listener = TcpListener::bind(addr).await.unwrap();

                serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .unwrap();
            });
        });

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(v) => {
                    if let Some(p) = procedures.get(&v) {
                        p.call()?;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if thandle.is_finished() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        if let Err(e) = thandle.join() {
            Err(format!("[WARNING] axum thread closed with panic: {:#?}", e))?;
        } else {
            println!("[INFO] all good, ejecting payload...");
        }

        syringe.eject(injected_payload)?;

        println!("[INFO] bye.")
    } else {
        eprintln!(
            "[ERROR] program whose name contains '{}' doesn't seem to be run...",
            target_name
        );
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
