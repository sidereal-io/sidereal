//! Configure and initialize PostgreSQL before binding any HTTP listener.
use sidereal_server::{app_with_database, config::DatabaseConfig};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = DatabaseConfig::from_env()?;
    let database = sidereal_core::database::initialize(
        config.options,
        config.startup_timeout,
        config.migration_timeout,
    )
    .await?;
    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(5000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            database.close().await;
            return Err(error.into());
        }
    };
    println!(
        "sidereal-server listening on {addr} (core abi {}, astro pack abi {})",
        sidereal_core::abi_version(),
        sidereal_pack_astro::abi_version()
    );
    let result = axum::serve(listener, app_with_database(database.clone()))
        .with_graceful_shutdown(shutdown())
        .await;
    database.close().await;
    result?;
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install termination handler");
        tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
