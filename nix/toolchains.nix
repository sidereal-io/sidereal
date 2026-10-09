# Rust, Node, pnpm and just. Specific to Sidereal (design.md D4).
{ ... }:
{
  perSystem = { config, pkgs, lib, ... }: {
    config.sidereal.shell.packages =
      let
        # The file names the Rust channel, which rustup also reads.
        # flake.lock decides the release: the newest one the locked
        # rust-overlay knows.
        rustToolchain =
          pkgs.rust-bin.fromRustupToolchainFile ../server/rust-toolchain.toml;
      in
      [
        rustToolchain
        pkgs.nodejs_26
        pkgs.pnpm_12
        pkgs.openssl
        pkgs.just
      ];
  };
}
