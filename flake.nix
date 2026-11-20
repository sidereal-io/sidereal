{
  description = "Sidereal development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    # Pinned to a release tag, so mise.toml can name the same release and
    # an update cannot land on an untagged commit. It stays pinned until
    # the Nix route goes. It does not follow our nixpkgs, which keeps its
    # bundle as the release built it. The shell's `openspec` comes from
    # nix/openspec.nix instead, ahead of the bundle's older copy.
    lets-spec.url = "github:sidereal-io/lets-spec/v0.8.0";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];

      imports = [
        ./nix/devshell.nix
        ./nix/toolchains.nix
        ./nix/lets-spec.nix
        ./nix/openspec.nix
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
