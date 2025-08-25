{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay";

    naersk.url = "github:nix-community/naersk";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    naersk,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        toolchain =
          pkgs.rust-bin.selectLatestNightlyWith
          (toolchain: toolchain.default);

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          rustc = toolchain;
        };
      in {
        packages.default = naersk'.buildPackage {
          pname = "kaka-nest";
          src = ./.;
        };

        devShells.default = pkgs.mkShell {
          # Additional dev-shell environment variables can be set directly
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            pkgs.hyperfine
            pkgs.linuxKernel.packages.linux_zen.perf
            pkgs.gnuplot
            toolchain
          ];
        };
      }
    );
}
