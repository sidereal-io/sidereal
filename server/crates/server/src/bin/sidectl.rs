//! `sidectl`: run on the box, with the server's database settings.
use sidereal_server::{
    config::DatabaseConfig,
    sidectl::{run, terminal::Console},
};
use std::{io, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    ExitCode::from(
        run(
            std::env::args().skip(1),
            DatabaseConfig::from_env,
            &mut Console::stdio(),
            &mut io::stdout(),
            &mut io::stderr(),
        )
        .await,
    )
}
