//! `sidereal-preview-worker`: makes one preview, in its own process, for
//! the server (ADR-020).
use sidereal_server::preview_worker::{self, BoundedAllocator};
use std::process::ExitCode;

#[global_allocator]
static ALLOCATOR: BoundedAllocator = BoundedAllocator;

fn main() -> ExitCode {
    preview_worker::main()
}
