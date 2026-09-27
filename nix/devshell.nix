# The shell itself: two options that any module (in this repo or, later, a
# shared one) can add to without editing this file. See design.md D4.
{ lib, ... }:
{
  perSystem = { config, pkgs, lib, ... }: {
    options.sidereal.shell = {
      packages = lib.mkOption {
        type = lib.types.listOf lib.types.package;
        default = [ ];
        description = "Packages merged into the development shell.";
      };

      hooks = lib.mkOption {
        type = lib.types.listOf lib.types.lines;
        default = [ ];
        description = ''
          Shell script fragments run, in order, when the development shell
          starts. This module adds the `just enter` hook last.
        '';
      };
    };

    config = {
      devShells.default = pkgs.mkShell {
        packages = config.sidereal.shell.packages;
        shellHook = lib.concatStringsSep "\n" config.sidereal.shell.hooks;
      };

      # Run the repo's `enter` recipe, if its justfile defines one, after every
      # other hook. The justfile decides what entering means. A failure never
      # stops the shell from loading.
      sidereal.shell.hooks = lib.mkAfter [
        ''
          { command -v just >/dev/null 2>&1 && just --show enter >/dev/null 2>&1 && just enter; } || true
        ''
      ];

      # Building the shell is also a flake check (design.md D8). Story E4
      # adds more checks here.
      checks.devshell = config.devShells.default;
    };
  };
}
