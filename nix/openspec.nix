# The openspec CLI. Nothing here is specific to Sidereal, so a later shared
# flake can take this module over unchanged (design.md D4, D5).
{ ... }:
{
  perSystem = { config, pkgs, ... }: {
    config.sidereal.shell.packages = [ pkgs.openspec ];
  };
}
