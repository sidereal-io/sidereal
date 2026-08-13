//! Sidereal v2 server binary.
//!
//! A thin shell that wires `core` and the compiled-in packs. The axum HTTP/WS
//! surface and `GET /healthz` land in #229; this M0 placeholder only proves the
//! binary builds and links its crates.

fn main() {
    println!(
        "sidereal-server placeholder (core abi {}, astro pack abi {})",
        sidereal_core::abi_version(),
        sidereal_pack_astro::abi_version(),
    );
}
