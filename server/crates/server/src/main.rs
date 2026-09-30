//! Sidereal v2 server binary.
//!
//! A thin shell that wires `core` and the compiled-in packs, then serves the
//! axum app. Real route wiring and pack registration land in later M0/M1 work;
//! for now it boots the app and serves `GET /healthz`.

use std::net::SocketAddr;

use sidereal_server::app;

/// Server port. Defaults to 5000 (see CLAUDE.md); override with `PORT`.
const DEFAULT_PORT: u16 = 5000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!(
        "sidereal-server listening on {addr} (core abi {}, astro pack abi {})",
        sidereal_core::abi_version(),
        sidereal_pack_astro::abi_version(),
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app()).await?;
    Ok(())
}
