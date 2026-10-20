//! Check the web and storage roots, then initialize PostgreSQL and pair
//! storage, before binding any HTTP listener.
use sidereal_core::database::{initialize, initialize_with_storage};
use sidereal_server::{
    app_with_database,
    config::{storage_root_from_env, web_root_from_env, DatabaseConfig},
    with_web_root,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // A start that cannot serve its web shell or reach its storage directory
    // must not change the database first.
    let web_root = web_root_from_env()?;
    let storage = storage_root_from_env()?;
    let config = DatabaseConfig::from_env()?;
    let (options, startup, migration) = (
        config.options,
        config.startup_timeout,
        config.migration_timeout,
    );
    let database = match &storage {
        Some(storage) => initialize_with_storage(options, startup, migration, storage).await?,
        None => initialize(options, startup, migration).await?,
    };
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
    let app = with_web_root(app_with_database(database.clone()), web_root.as_deref());
    let result = axum::serve(listener, app)
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
