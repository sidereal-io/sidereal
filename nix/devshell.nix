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
          starts. Empty in this change; story E3 adds the first hook.
        '';
      };
    };

    config = {
      devShells.default = pkgs.mkShell {
        packages = config.sidereal.shell.packages;
        shellHook = lib.concatStringsSep "\n" config.sidereal.shell.hooks;
      };

      # Building the shell is also a flake check (design.md D8). Story E4
      # adds more checks here.
      checks.devshell = config.devShells.default;
    };
  };
}
