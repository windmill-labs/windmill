{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs-oapi-gen.url =
      "nixpkgs/2d068ae5c6516b2d04562de50a58c682540de9bf"; # openapi-generator-cli pin to 7.10.0
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay
    , nixpkgs-oapi-gen }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ (import rust-overlay) ];
        };

        openapi-generator-cli =
          (import nixpkgs-oapi-gen { inherit system; }).openapi-generator-cli;

        lib = pkgs.lib;
        stdenv = pkgs.stdenv;
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src" # for rust-analyzer
            "rust-analyzer"
            "rustfmt"
          ];
        };
        patchedClang = pkgs.llvmPackages_18.clang.overrideAttrs (oldAttrs: {
          postFixup = ''
            # Copy the original postFixup logic but skip add-hardening.sh
            ${oldAttrs.postFixup or ""}

            # Remove the line that substitutes add-hardening.sh
            sed -i 's/.*source.*add-hardening\.sh.*//' $out/bin/clang
          '';
        });
        buildInputs = with pkgs; [
          openssl
          openssl.dev
          libxml2.dev
          xmlsec.dev
          libxslt.dev
          libclang.dev
          curl.dev
          zlib.dev
          libffi # For deno_ffi
          libtool
          nodejs
          postgresql
          pkg-config
          llvmPackages_18.clang
          mold
          cmake
          cyrus_sasl
          krb5
        ];
        coursier = pkgs.fetchFromGitHub {
          owner = "coursier";
          repo = "launchers";
          rev = "79d927f7586c09ca6d8cd01862adb0d9f9d88dff";
          hash = "sha256-8E0WtDFc7RcqmftDigMyy1xXUkjgL4X4kpf7h1GdE48=";
        };

        PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig"
          (with pkgs; [ openssl.dev libxml2.dev xmlsec.dev libxslt.dev cyrus_sasl.dev krb5.dev ]);
        RUSTY_V8_ARCHIVE = let
          # NOTE: needs to be same as in Cargo.toml
          version = "130.0.7";
          target = pkgs.hostPlatform.rust.rustcTarget;
          sha256 = {
            x86_64-linux =
              "sha256-pkdsuU6bAkcIHEZUJOt5PXdzK424CEgTLXjLtQ80t10=";
            aarch64-linux = pkgs.lib.fakeHash;
            x86_64-darwin = pkgs.lib.fakeHash;
            aarch64-darwin = pkgs.lib.fakeHash;
          }.${system};
        in pkgs.fetchurl {
          name = "librusty_v8-${version}";
          url =
            "https://github.com/denoland/rusty_v8/releases/download/v${version}/librusty_v8_release_${target}.a.gz";
          inherit sha256;
        };
      in {
        # Enter by `nix develop .#wasm`
        devShells."wasm" = pkgs.mkShell {
          # Explicitly set paths for headers and linker
          shellHook = ''
            export CC=${patchedClang}/bin/clang
          '';
          buildInputs = buildInputs ++ (with pkgs; [
            (rust-bin.nightly.latest.default.override {
              extensions = [
                "rust-src" # for rust-analyzer
                "rust-analyzer"
              ];
              targets =
                [ "wasm32-unknown-unknown" "wasm32-unknown-emscripten" ];
            })
            wasm-pack
            deno
            emscripten
            nushell
            # Needed for extra dependencies
            glibc_multi
          ]);
        };
        devShells."cli" = pkgs.mkShell {
          shellHook = ''
            if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
              export FLAKE_ROOT="$(git rev-parse --show-toplevel)"
            else
              # Fallback to PWD if not in a git repository
              export FLAKE_ROOT="$PWD"
            fi
            wm-cli-deps
          '';
          buildInputs = buildInputs ++ [ pkgs.deno ];
          packages = [
            (pkgs.writeScriptBin "wm-cli" ''
              deno run -A --no-check $FLAKE_ROOT/cli/src/main.ts $*
            '')
            (pkgs.writeScriptBin "wm-cli-deps" ''
              pushd $FLAKE_ROOT/cli/
              ${if pkgs.stdenv.isDarwin then
                "./gen_wm_client_mac.sh && ./windmill-utils-internal/gen_wm_client_mac.sh"
              else
                "./gen_wm_client.sh && ./windmill-utils-internal/gen_wm_client.sh"}
              popd
            '')
          ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = buildInputs ++ [
            # To update run: `nix flake update nixpkgs-oapi-gen`
            openapi-generator-cli
          ] ++ (with pkgs; [
            # Essentials
            rust
            git
            sqlx-cli

            # Build/helper scripts
            jq
            gnused # other implementations are inconsistent on osx

            # Python
            flock
            python3
            python3Packages.pip
            uv
            poetry
            pyright
            openapi-python-client

            # Other languages
            deno
            typescript
            nushell
            go
            bun
            dotnet-sdk_9
            oracle-instantclient
            ansible
            ruby_3_4
            cargo-sweep # We use it for rust

            # LSP/Local dev
            svelte-language-server
            taplo

            # Orchestration/Kubernetes
            minikube
            kubectl
            kubernetes-helm
            conntrack-tools # To run minikube without driver (--driver=none)
            cri-tools

            # Extra
            xcaddy
            cargo-watch
            nsjail
            sccache
          ]);
          packages = [
            (pkgs.writeScriptBin "wm-caddy" ''
              cd ./frontend
              xcaddy build $* \
                --with github.com/mholt/caddy-l4@145ec36251a44286f05a10d231d8bfb3a8192e09 \
                --with github.com/RussellLuo/caddy-ext/layer4@ab1e18cfe426012af351a68463937ae2e934a2a1
            '')
            (pkgs.writeScriptBin "wm-build" ''
              cd ./frontend
              npm install
              npm run ${
                if pkgs.stdenv.isDarwin then
                  "generate-backend-client-mac"
                else
                  "generate-backend-client"
              }
              npm run build $*
            '')
            (pkgs.writeScriptBin "wm-migrate" ''
              cd ./backend
              sqlx migrate run
            '')
            (pkgs.writeScriptBin "wm-setup" ''
              sqlx database create
              wm-build
              wm-caddy
              wm-migrate
            '')
            (pkgs.writeScriptBin "wm-reset" ''
              sqlx database drop -f
              sqlx database create
              wm-migrate
            '')
            (pkgs.writeScriptBin "wm-bench" ''
              deno run -A benchmarks/main.ts -e admin@windmill.dev -p changeme $*
            '')
            (pkgs.writeScriptBin "wm" ''
              cd ./frontend
              npm install
              npm run generate-backend-client
              npm run dev $*
            '')
            (pkgs.writeScriptBin "wm-minio" ''
              set -e
              cd ./backend
              mkdir -p .minio-data/wmill
              ${pkgs.minio}/bin/minio server ./.minio-data --console-address ":9001"
            '')
            # Generate keys
            # TODO: Do not set new keys if ran multiple times
            (pkgs.writeScriptBin "wm-minio-keys" ''
              set -e
              cd ./backend

              # Set up MinIO alias
              ${pkgs.minio-client}/bin/mc alias set 'wmill-minio-dev' 'http://localhost:9000' 'minioadmin' 'minioadmin'

              # Check if secrets file exists and contains valid keys
              if [[ -f .minio-data/secrets.txt ]] && [[ -s .minio-data/secrets.txt ]]; then
                  echo "Access keys already exist:"
                  cat .minio-data/secrets.txt
                  echo ""
                  echo "Keys loaded from: ./backend/.minio-data/secrets.txt"
              else
                  echo "Creating new access keys..."
                  mkdir -p .minio-data
                  ${pkgs.minio-client}/bin/mc admin accesskey create 'wmill-minio-dev' | tee .minio-data/secrets.txt
                  echo ""
                  echo 'New keys saved to: ./backend/.minio-data/secrets.txt'
              fi

              echo "bucket: wmill"
              echo "endpoint: http://localhost:9000"
            '')
          ];

          inherit PKG_CONFIG_PATH RUSTY_V8_ARCHIVE;
          GIT_PATH = "${pkgs.git}/bin/git";
          NODE_ENV = "development";
          NODE_OPTIONS = "--max-old-space-size=16384";
          # DATABASE_URL = "postgres://postgres:changeme@127.0.0.1:5432/";
          DATABASE_URL =
            "postgres://postgres:changeme@127.0.0.1:5432/windmill?sslmode=disable";

          REMOTE = "http://127.0.0.1:8000";
          REMOTE_LSP = "http://127.0.0.1:3001";
          # RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
          DENO_PATH = "${pkgs.deno}/bin/deno";
          GO_PATH = "${pkgs.go}/bin/go";
          PHP_PATH = "${pkgs.php}/bin/php";
          COMPOSER_PATH = "${pkgs.php84Packages.composer}/bin/composer";
          BUN_PATH = "${pkgs.bun}/bin/bun";
          NODE_PATH = "${pkgs.nodejs}/bin/node";
          NODE_BIN_PATH = "${pkgs.nodejs}/bin/node";
          UV_PATH = "${pkgs.uv}/bin/uv";
          NU_PATH = "${pkgs.nushell}/bin/nu";
          JAVA_PATH = "${pkgs.jdk21}/bin/java";
          JAVAC_PATH = "${pkgs.jdk21}/bin/javac";
          COURSIER_PATH = "${coursier}/coursier";
          BASH_PATH = "bash";
          RUBY_PATH = "${pkgs.ruby}/bin/ruby";
          RUBY_BUNDLE_PATH = "${pkgs.ruby}/bin/bundle";
          RUBY_GEM_PATH = "${pkgs.ruby}/bin/gem";
          # for related places search: ADD_NEW_LANG 
          FLOCK_PATH = "${pkgs.flock}/bin/flock";
          CARGO_PATH = "${rust}/bin/cargo";
          CARGO_SWEEP_PATH = "${pkgs.cargo-sweep}/bin/cargo-sweep";
          DOTNET_PATH = "${pkgs.dotnet-sdk_9}/bin/dotnet";
          DOTNET_ROOT = "${pkgs.dotnet-sdk_9}/share/dotnet";
          ORACLE_LIB_DIR = "${pkgs.oracle-instantclient.lib}/lib";
          ANSIBLE_PLAYBOOK_PATH = "${pkgs.ansible}/bin/ansible-playbook";
          ANSIBLE_GALAXY_PATH = "${pkgs.ansible}/bin/ansible-galaxy";
          # RUST_LOG = "debug";
          # RUST_LOG = "kube=debug";

          # Override cargo linker to use clang 18 (stdenv brings clang 21 which causes SIGSEGV with mold)
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.llvmPackages_18.clang}/bin/clang";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.llvmPackages_18.clang}/bin/clang";
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold -C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath [ pkgs.openssl pkgs.libffi pkgs.cyrus_sasl pkgs.krb5 pkgs.libxml2 pkgs.xmlsec pkgs.libxslt stdenv.cc.cc.lib ]}";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold -C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath [ pkgs.openssl pkgs.libffi pkgs.cyrus_sasl pkgs.krb5 pkgs.libxml2 pkgs.xmlsec pkgs.libxslt stdenv.cc.cc.lib ]}";
          # rpath for build scripts and proc macros (host compilation)
          CARGO_HOST_RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath [ pkgs.openssl pkgs.libffi pkgs.cyrus_sasl pkgs.krb5 pkgs.libxml2 pkgs.xmlsec pkgs.libxslt stdenv.cc.cc.lib ]}";

          # See this issue: https://github.com/NixOS/nixpkgs/issues/370494
          # Allows to build jemalloc on nixos
          CFLAGS = "-Wno-error=int-conversion";

          # Need to tell bindgen where to find libclang
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          # LD_LIBRARY_PATH set in shellHook with a wrapper to avoid leaking into git/ssh
          # LD_LIBRARY_PATH = "${pkgs.gcc.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.zlib
          ];

          # Set C flags for Rust's bindgen program. Unlike ordinary C
          # compilation, bindgen does not invoke $CC directly. Instead it
          # uses LLVM's libclang. To make sure all necessary flags are
          # included we need to look in a few places.
          # See https://web.archive.org/web/20220523141208/https://hoverbear.org/blog/rust-bindgen-in-nix/
          BINDGEN_EXTRA_CLANG_ARGS =
            # Prevent clang from using system headers - only use Nix headers
            "-nostdinc ${
              builtins.readFile "${stdenv.cc}/nix-support/libc-crt1-cflags"
            } ${builtins.readFile "${stdenv.cc}/nix-support/libc-cflags"} ${
              builtins.readFile "${stdenv.cc}/nix-support/cc-cflags"
            } ${
              builtins.readFile "${stdenv.cc}/nix-support/libcxx-cxxflags"
            } -idirafter ${pkgs.libiconv}/include ${
              lib.optionalString stdenv.cc.isClang
              "-idirafter ${stdenv.cc.cc}/lib/clang/${
                lib.getVersion stdenv.cc.cc
              }/include"
            }${
              lib.optionalString stdenv.cc.isGNU
              "-isystem ${stdenv.cc.cc}/include/c++/${
                lib.getVersion stdenv.cc.cc
              } -isystem ${stdenv.cc.cc}/include/c++/${
                lib.getVersion stdenv.cc.cc
              }/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${
                lib.getVersion stdenv.cc.cc
              }/include"
            }";
        };
        packages.default = self.packages.${system}.windmill;
        packages.windmill-client = pkgs.buildNpmPackage {
          name = "windmill-client";
          version = (pkgs.lib.strings.trim (builtins.readFile ./version.txt));

          src = pkgs.nix-gitignore.gitignoreSource [ ] ./frontend;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ nodejs pixman cairo pango ];
          doCheck = false;

          npmDepsHash = "sha256-NXk9mnf74+/k0i3goqU8Zi/jr5b/bmW+HWRLJCI2CX8=";
          npmBuild = "npm run build";

          postUnpack = ''
            mkdir -p ./backend/windmill-api/
            cp ${
              ./backend/windmill-api/openapi.yaml
            } ./backend/windmill-api/openapi.yaml
            cp ${./openflow.openapi.yaml} ./openflow.openapi.yaml
          '';
          preBuild = ''
            npm run ${
              if pkgs.stdenv.isDarwin then
                "generate-backend-client-mac"
              else
                "generate-backend-client"
            }
          '';

          installPhase = ''
            mkdir -p $out/build
            cp -r build $out
          '';

          NODE_OPTIONS = "--max-old-space-size=8192";
        };
        packages.windmill = pkgs.rustPlatform.buildRustPackage {
          pname = "windmill";
          version = (pkgs.lib.strings.trim (builtins.readFile ./version.txt));

          src = ./backend;
          nativeBuildInputs = buildInputs
            ++ [ self.packages.${system}.windmill-client pkgs.perl ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];

          cargoLock = {
            lockFile = ./backend/Cargo.lock;
            outputHashes = {
              "php-parser-rs-0.1.3" =
                "sha256-ZeI3KgUPmtjlRfq6eAYveqt8Ay35gwj6B9iOQRjQa9A=";
              "progenitor-0.3.0" =
                "sha256-F6XRZFVIN6/HfcM8yI/PyNke45FL7jbcznIiqj22eIQ=";
              "tinyvector-0.1.0" =
                "sha256-NYGhofU4rh+2IAM+zwe04YQdXY8Aa4gTmn2V2HtzRfI=";
            };
          };

          buildFeatures = [
            "enterprise"
            "enterprise_saml"
            "stripe"
            "embedding"
            "parquet"
            "prometheus"
            "openidconnect"
            "cloud"
            "jemalloc"
            "tantivy"
            "license"
            "http_trigger"
            "zip"
            "oauth2"
            "kafka"
            "otel"
            "dind"
            "websocket"
            "smtp"
            "static_frontend"
            "all_languages"
          ];
          doCheck = false;

          inherit PKG_CONFIG_PATH RUSTY_V8_ARCHIVE;
          SQLX_OFFLINE = true;
          FRONTEND_BUILD_DIR =
            "${self.packages.${system}.windmill-client}/build";
        };
      });
}
