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

      # End-to-end test driver. Pinned to the same version as the
      # `@playwright/test` entry in web/package.json; `just e2e` refuses to
      # run if the two drift apart.
      playwrightVersion = "1.64.1";
    in
    {
      config.sidereal.shell.packages =
        [
          rustToolchain
          pkgs.nodejs_26
          pkgs.pnpm_12
          pkgs.openssl
          pkgs.just
          # scripts/board-sync.sh reads the project board with it.
          pkgs.jq
          # End-to-end tests (`just e2e`). The driver supplies the
          # Playwright runtime; the browsers attribute below supplies a
          # closed-store Chromium so nothing is downloaded on first run.
          pkgs.playwright-driver
          pkgs.playwright-driver.browsers-chromium
        ];

      config.sidereal.shell.env = {
        # `playwright` from Nix sets this for its own wrapper, but `pnpm e2e`
        # runs the copy resolved out of web/node_modules, which never sees
        # that wrapper. Without this the npm CLI tries to download its own
        # Chromium and fails inside `nix develop`.
        PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers-chromium}";
        # Read by the `just e2e` recipe to check the Nix-pinned driver still
        # matches the version web/package.json asks for.
        SIDEREAL_PLAYWRIGHT_VERSION = playwrightVersion;
      };
    };
}