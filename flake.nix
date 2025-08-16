{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
        };

        kaka-nest = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          }
        );
      in
      {
        checks = {
          inherit kaka-nest;
        };

        packages.default = kaka-nest;

        apps.default = flake-utils.lib.mkApp {
          drv = kaka-nest;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            # pkgs.sqlx-cli
            # pkgs.sqlite
            # pkgs.ripgrep
          ];
        };
      }
    );
}
