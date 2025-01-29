{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      rust = pkgs.rust-bin.nightly.latest.default.override {
        extensions = [
          "rust-src"
        ];
        targets = ["wasm32-unknown-unknown"];
      };
    in {
      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          rust
          nodejs
          wasm-pack
          sccache
        ];
        RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        CARGO_PATH = "${rust}/bin/cargo";
      };
    });
}
