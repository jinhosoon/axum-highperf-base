{
  description = "axum-highperf-base: High-performance Rust (Axum) backend with 64-Shard RwLock, Inverted Index + Okapi BM25 search, and Argo CD GitOps";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "rust-api";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          meta = with pkgs.lib; {
            description = "High-performance Rust Axum backend with Inverted Index + Okapi BM25 search";
            mainProgram = "rust-api";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            pkg-config
          ];

          RUST_LOG = "rust_api=debug,tower_http=debug";
        };
      }
    );
}
