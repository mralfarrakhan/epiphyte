use std::{error::Error, net::SocketAddr, sync::mpsc, thread, time::Duration};

use axum::{Router, response::Json, routing::get, serve};
use dll_syringe::{
    Syringe,
    process::{OwnedProcess, Process},
};
use serde_json::json;
use tokio::{net::TcpListener, runtime::Builder, signal};

mod config;
mod payload;

enum Command {
    OffsetCmd,
}

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
            "injected process base name: {}, path: {}, pid: {}.",
            base_name, exec_path, pid
        );

        if options.is_verbose {
            println!();
            payload::print_symbol_table(&procedures)?;
            println!("only symbols with both path and address are accessible");
            println!();
        }

        println!("REST procedure call available at port {}", port,);

        let syringe = Syringe::for_process(target_process);
        let injected_payload = syringe.inject(payload_path)?;

        let (cmd_tx, cmd_rx) = mpsc::channel();

        let info = async move || {
            Json(json!({
                "base_name": base_name,
                "exec_path": exec_path,
                "pid": pid,
            }))
        };

        let offset_tx = cmd_tx.clone();
        let thandle = thread::spawn(move || {
            let runtime = Builder::new_current_thread().enable_all().build().unwrap();

            runtime.block_on(async {
                let app = Router::new().route("/", get(info)).route(
                    "/offset",
                    get(async move || offset_tx.send(Command::OffsetCmd).unwrap()),
                );

                let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
                let listener = TcpListener::bind(addr).await.unwrap();

                serve(listener, app)
                    .with_graceful_shutdown(shutdown_signal())
                    .await
                    .unwrap();
            });
        });

        let remote_offset = unsafe {
            syringe.get_raw_procedure::<extern "system" fn()>(injected_payload, "offset")
        }?
        .ok_or("error fetching remote procedure")?;

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Command::OffsetCmd) => remote_offset.call()?,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if thandle.is_finished() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        if let Err(e) = thandle.join() {
            Err(format!("axum thread closed with panic: {:#?}", e))?;
        } else {
            println!("All good. Ejecting payload...");
        }

        syringe.eject(injected_payload)?;
    } else {
        eprintln!(
            "program whose name contains '{}' doesn't seem to be run...",
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
