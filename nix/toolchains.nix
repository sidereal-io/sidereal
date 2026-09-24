# Rust, Node and just. Specific to Sidereal (design.md D4).
{ ... }:
{
  perSystem = { config, pkgs, lib, ... }: {
    config.sidereal.shell.packages =
      let
        # The only source of truth for the Rust version (design.md D3):
        # rustup reads the same file.
        rustToolchain =
          pkgs.rust-bin.fromRustupToolchainFile ../backend/rust-toolchain.toml;
      in
      [
        rustToolchain
        pkgs.nodejs_26
        pkgs.just
      ];
  };
}
