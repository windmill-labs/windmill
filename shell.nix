{ pkgs ? import <nixpkgs> { } }:

/* based on
   https://discourse.nixos.org/t/how-can-i-set-up-my-rust-programming-environment/4501/9
*/
let
  rust_overlay = import (builtins.fetchTarball
    "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  lib = pkgs.lib;
  stdenv = pkgs.stdenv;
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
    # uv
    python312Packages.pip-tools # pip-compile
  ];

  # buildInputs = with pkgs; [ xz lzma ];

  # Add the following lines to set the LD_LIBRARY_PATH
  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (with pkgs; [
    xz
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

  # ---- Samael ----
  # https://github.com/njaremko/samael/blob/master/flake.nix#L104-L119
  # Otherwise samael crate will fail to build
  # Need to tell bindgen where to find libclang
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  # Set C flags for Rust's bindgen program. Unlike ordinary C
  # compilation, bindgen does not invoke $CC directly. Instead it
  # uses LLVM's libclang. To make sure all necessary flags are
  # included we need to look in a few places.
  # See https://web.archive.org/web/20220523141208/https://hoverbear.org/blog/rust-bindgen-in-nix/
  BINDGEN_EXTRA_CLANG_ARGS =
    "${builtins.readFile "${stdenv.cc}/nix-support/libc-crt1-cflags"} ${
      builtins.readFile "${stdenv.cc}/nix-support/libc-cflags"
    } ${builtins.readFile "${stdenv.cc}/nix-support/cc-cflags"} ${
      builtins.readFile "${stdenv.cc}/nix-support/libcxx-cxxflags"
    } -idirafter ${pkgs.libiconv}/include ${
      lib.optionalString stdenv.cc.isClang
      "-idirafter ${stdenv.cc.cc}/lib/clang/${
        lib.getVersion stdenv.cc.cc
      }/include"
    } ${
      lib.optionalString stdenv.cc.isGNU
      "-isystem ${stdenv.cc.cc}/include/c++/${
        lib.getVersion stdenv.cc.cc
      } -isystem ${stdenv.cc.cc}/include/c++/${
        lib.getVersion stdenv.cc.cc
      }/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${
        lib.getVersion stdenv.cc.cc
      }/include"
    }";
  # ---- Samael ----
}
