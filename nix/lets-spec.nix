# Let's Spec: the OpenSpec CLI, the `lets` command, `gh`, and the shell hook
# that runs `lets sync` so the agent skills, schemas and working rules match
# the release flake.lock pins. See https://github.com/sidereal-io/lets-spec.
{ inputs, ... }:
{
  imports = [ inputs.lets-spec.flakeModules.default ];

  perSystem = { config, ... }: {
    letsSpec = {
      flow = "standard";
      skills = "committed";
    };

    sidereal.shell = {
      packages = config.letsSpec.packages;
      hooks = [ config.letsSpec.shellHook ];
    };
  };
}
