{ pkgs ? import <nixpkgs> { } }:

/* based on
   https://discourse.nixos.org/t/how-can-i-set-up-my-rust-programming-environment/4501/9
*/
let
  rust_overlay = import (builtins.fetchTarball
    "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  # TODO: Pin version?
  # rustVersion = "latest";
  rustVersion = "2024-09-30";
  rust = pkgs.rust-bin.nightly.${rustVersion}.default.override {
    extensions = [
      "rust-src" # for rust-analyzer
      "rust-analyzer"
    ];
  };
in pkgs.mkShell {

  packages = with pkgs; [
    rust
    # rustup
    cargo-watch
    typescript # tsc
    typescript-language-server
    postgresql
    watchexec # used in client's dev.nu
    poetry # for python client
    uv
    python312Packages.pip-tools # pip-compile
  ];

  # buildInputs = with pkgs; [ xz lzma ];

  # Add the following lines to set the LD_LIBRARY_PATH
  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (with pkgs; [
    lzma
    libseccomp
    bzip2
    openssl_3_3
    #
  ])}";

  REMOTE = "http://127.0.0.1:8000";
  REMOTE_LSP = "http://127.0.0.1:3001";

  DATABASE_URL =
    "postgres://postgres:changeme@127.0.0.1:5432/windmill?sslmode=disable";

  RUSTC_LINKER = "${pkgs.clang}/bin/clang";
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.clang}/bin/clang";

  RUSTFLAGS =
    "-C link-arg=-fuse-ld=${pkgs.mold}/bin/mold -Zshare-generics=y -Z threads=4";
  RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";

  # Useful for development
  RUST_LOG = "debug";

}
