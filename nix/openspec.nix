# The OpenSpec CLI, pinned on its own until a Let's Spec release names
# OpenSpec 2. The Let's Spec bundle still carries an older `openspec`, so
# this copy goes first on the shell's PATH.
{ ... }:
{
  perSystem = { config, pkgs, lib, system, ... }:
    let
      version = "2.1.1";

      # Hashes from the release's SHA256SUMS, converted to SRI form with
      # `nix hash convert --hash-algo sha256 --to sri <hex>`.
      assets = {
        x86_64-linux = {
          file = "openspec-linux-x64.tar.gz";
          hash = "sha256-IvDHyWzBQTi+y0KqmOWJBQyUzaeTy+/X56wtfe54ttU=";
        };
        aarch64-linux = {
          file = "openspec-linux-arm64.tar.gz";
          hash = "sha256-t6sW5BOOiN8tLdwT5oe/IPb+InUdtoSS1I4A13sBg9o=";
        };
        aarch64-darwin = {
          file = "openspec-darwin-arm64.tar.gz";
          hash = "sha256-DNLGpbMejSi5ej9aZExTPdDctVEBYu/ive62eHekHYM=";
        };
      };

      asset = assets.${system};

      openspec = pkgs.stdenv.mkDerivation {
        pname = "openspec";
        inherit version;

        src = pkgs.fetchurl {
          url = "https://github.com/Fission-AI/OpenSpec/releases/download/v${version}/${asset.file}";
          inherit (asset) hash;
        };

        # The tarball holds the binary at its top level, with no folder.
        sourceRoot = ".";

        nativeBuildInputs = [ pkgs.makeWrapper ]
          ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ];

        # Bun-compiled: the program is appended to the executable, and
        # stripping would cut it off.
        dontStrip = true;

        installPhase = ''
          runHook preInstall
          install -Dm755 openspec $out/libexec/openspec
          makeWrapper $out/libexec/openspec $out/bin/openspec \
            --set OPENSPEC_NO_UPDATE_CHECK 1
          runHook postInstall
        '';
      };
    in
    {
      sidereal.shell.packages = lib.mkBefore [ openspec ];

      # Catches another package's `openspec` shadowing this one. Build
      # inputs go on PATH in list order, as they do in the shell.
      checks.openspec-version = pkgs.runCommand "openspec-version"
        { nativeBuildInputs = config.sidereal.shell.packages; }
        ''
          export HOME=$TMPDIR
          actual=$(openspec --version)
          if [ "$actual" != "${version}" ]; then
            echo "The shell's openspec --version prints $actual, but nix/openspec.nix pins ${version}." >&2
            exit 1
          fi
          touch $out
        '';
    };
}
