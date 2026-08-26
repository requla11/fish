//! rust-service — TCP echo service of the polyglot demo stack.
//!
//! Contract-first: the TaskEvent schema is owned by py-worker and embedded at
//! COMPILE time via include_str!. If the contract file moves, this crate stops
//! building — the strongest form of cross-project dependency, which `fish
//! build` infers automatically from this source reference.

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The shared event contract, relative to this file:
/// src/main.rs → ../../ → demo root → py-worker/contracts/.
const EVENTS_SCHEMA: &str = include_str!("../../py-worker/contracts/events.schema.json");

fn contract_title() -> String {
    serde_json::from_str::<Value>(EVENTS_SCHEMA)
        .ok()
        .and_then(|schema| schema["title"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "<invalid>".to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "🦀 Rust Service starting on port 8080 (contract: {})...",
        contract_title()
    );

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("✅ Rust Service listening on 127.0.0.1:8080");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read from socket: {}", e);
                        return;
                    }
                };

                let response = format!(
                    "Rust Service [contract: {}] received: {}",
                    contract_title(),
                    String::from_utf8_lossy(&buf[..n])
                );
                if let Err(e) = socket.write_all(response.as_bytes()).await {
                    eprintln!("Failed to write to socket: {}", e);
                    return;
                }
            }
        });
    }
}
