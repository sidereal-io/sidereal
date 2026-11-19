{
  description = "Sidereal development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    # Pinned to a release tag, so mise.toml can name the same release and
    # an update cannot land on an untagged commit. It stays pinned until
    # the Nix route goes. Its own nixpkgs pins the OpenSpec CLI the
    # release's openspec-* skills were generated from, so it does not
    # follow ours.
    lets-spec.url = "github:sidereal-io/lets-spec/v0.8.0";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];

      imports = [
        ./nix/devshell.nix
        ./nix/toolchains.nix
        ./nix/lets-spec.nix
      ];

      # Every module below gets a `pkgs` with the rust-overlay applied, so
      # nix/toolchains.nix can reach `pkgs.rust-bin` without redoing this.
      perSystem = { system, ... }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ inputs.rust-overlay.overlays.default ];
        };
      };
    };
}
