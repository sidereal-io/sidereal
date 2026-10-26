# Rust, Node, pnpm, just and jq. Specific to Sidereal (design.md D4).
{ ... }:
{
  perSystem = { config, pkgs, lib, ... }:
    let
      # The file names the Rust channel, which rustup also reads.
      # flake.lock decides the release: the newest one the locked
      # rust-overlay knows.
      rustToolchain =
        pkgs.rust-bin.fromRustupToolchainFile ../server/rust-toolchain.toml;
    in
    {
      # One owner per layer: Nix provides the runtimes, pnpm provides the
      # libraries, and Playwright provides its own browser. Pinning the
      # Playwright driver here as well meant a weekly Dependabot bump of
      # flake.lock could move the driver out of step with web/package.json and
      # break `e2e` until someone hand-edited both.
      config.sidereal.shell.packages =
        [
          rustToolchain
          pkgs.nodejs_26
          pkgs.pnpm_12
          pkgs.openssl
          pkgs.just
          # scripts/board-sync.sh reads the project board with it.
          pkgs.jq
        ];
    };
}
