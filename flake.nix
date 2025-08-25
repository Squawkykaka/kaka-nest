{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
            # NB: we disable tests since we'll run them all via cargo-nextest
            doCheck = false;
            pname = "kaka-nest-workspace";
          };

        fileSetForCrate = crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              (craneLib.fileset.commonCargoSources ./kaka-nest)
              (craneLib.fileset.commonCargoSources crate)
            ];
          };

        kaka-nest = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "kaka-nest";
            cargoExtraArgs = "-p kaka-nest";
            src = fileSetForCrate ./kaka-nest;
          }
        );
      in {
        checks = {
          inherit kaka-nest;

          workspace-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              cargoNextestPartitionsExtraArgs = "--no-tests=pass";
            }
          );
        };

        packages = {
          inherit kaka-nest;
          default = kaka-nest;
        };

        apps = {
          kaka-nest = flake-utils.lib.mkApp {
            drv = kaka-nest;
          };
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
            pkgs.hyperfine
            pkgs.linuxKernel.packages.linux_zen.perf
            pkgs.gnuplot
            pkgs.sqlx-cli
          ];
        };
      }
    );
}
