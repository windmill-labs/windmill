{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    # Pin openapi-generator-cli to 7.10.0
    nixpkgs-oapi-gen.url = "nixpkgs/2d068ae5c6516b2d04562de50a58c682540de9bf";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, nixpkgs-oapi-gen }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ (import rust-overlay) ];
        };

        lib = pkgs.lib;
        stdenv = pkgs.stdenv;

        # ---------------------------------------------------------------
        # Rust toolchain
        # ---------------------------------------------------------------

        rustStable = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" ];
        };

        # ---------------------------------------------------------------
        # Native C/C++ dependencies (required to compile the backend)
        # ---------------------------------------------------------------

        nativeBuildDeps = with pkgs; [
          # Crypto / TLS
          openssl
          openssl.dev

          # XML / SAML (enterprise_saml feature)
          libxml2.dev
          xmlsec.dev
          libxslt.dev

          # FFI / codegen
          libclang.dev
          libffi # deno_ffi on macOS

          # Networking / compression
          curl.dev
          zlib.dev

          # Auth (kafka-gssapi, mssql-kerberos)
          cyrus_sasl
          krb5

          # Misc
          libtool
          postgresql

          # Build tooling
          pkg-config
          llvmPackages_18.clang # linker — pinned to 18 to avoid SIGSEGV with mold + newer clang
          mold
          cmake # required by rdkafka cmake-build
        ];

        # ---------------------------------------------------------------
        # Prebuilt V8 binary (must match version in Cargo.toml)
        # ---------------------------------------------------------------

        rustyV8Archive = let
          version = "130.0.7";
          target = stdenv.hostPlatform.rust.rustcTarget;
          sha256 = {
            x86_64-linux = "sha256-pkdsuU6bAkcIHEZUJOt5PXdzK424CEgTLXjLtQ80t10=";
            aarch64-linux = lib.fakeHash;
            x86_64-darwin = lib.fakeHash;
            aarch64-darwin = lib.fakeHash;
          }.${system};
        in pkgs.fetchurl {
          name = "librusty_v8-${version}";
          url = "https://github.com/denoland/rusty_v8/releases/download/v${version}/librusty_v8_release_${target}.a.gz";
          inherit sha256;
        };

        # ---------------------------------------------------------------
        # pkg-config search path for native libraries
        # ---------------------------------------------------------------

        pkgConfigPath = lib.makeSearchPath "lib/pkgconfig"
          (with pkgs; [ openssl.dev libxml2.dev xmlsec.dev libxslt.dev cyrus_sasl.dev krb5.dev ]);

        # ---------------------------------------------------------------
        # RPATH — embed Nix store library paths into compiled binaries
        # ---------------------------------------------------------------

        rpathLibs = lib.makeLibraryPath (with pkgs; [
          openssl libffi cyrus_sasl krb5 libxml2 xmlsec libxslt stdenv.cc.cc.lib
        ]);

        # ---------------------------------------------------------------
        # Bindgen configuration
        # Bindgen uses libclang directly (not $CC), so we must explicitly
        # provide all Nix header search paths.
        # See: https://web.archive.org/web/20220523141208/https://hoverbear.org/blog/rust-bindgen-in-nix/
        # ---------------------------------------------------------------

        bindgenClangArgs = builtins.concatStringsSep " " ([
          "-nostdinc"
          (builtins.readFile "${stdenv.cc}/nix-support/libc-crt1-cflags")
          (builtins.readFile "${stdenv.cc}/nix-support/libc-cflags")
          (builtins.readFile "${stdenv.cc}/nix-support/cc-cflags")
          (builtins.readFile "${stdenv.cc}/nix-support/libcxx-cxxflags")
          "-idirafter ${pkgs.libiconv}/include"
        ] ++ lib.optionals stdenv.cc.isClang [
          "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"
        ] ++ lib.optionals stdenv.cc.isGNU [
          "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}"
          "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config}"
          "-idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"
        ]);

        # ---------------------------------------------------------------
        # Build environment variables (shared by all shells that compile Rust)
        # ---------------------------------------------------------------

        buildEnvVars = {
          PKG_CONFIG_PATH = pkgConfigPath;
          RUSTY_V8_ARCHIVE = rustyV8Archive;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = bindgenClangArgs;

          # Force clang 18 as cargo linker (stdenv may bring a newer clang that causes SIGSEGV with mold)
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.llvmPackages_18.clang}/bin/clang";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.llvmPackages_18.clang}/bin/clang";

          # Embed rpath so binaries find Nix store .so files at runtime
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold -C link-arg=-Wl,-rpath,${rpathLibs}";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=mold -C link-arg=-Wl,-rpath,${rpathLibs}";
          CARGO_HOST_RUSTFLAGS = "-C link-arg=-Wl,-rpath,${rpathLibs}";

          # https://github.com/NixOS/nixpkgs/issues/370494 — jemalloc build fix
          CFLAGS = "-Wno-error=int-conversion";

          LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.zlib ];
        };

        # ---------------------------------------------------------------
        # OpenAPI generator (pinned)
        # ---------------------------------------------------------------

        openapi-generator-cli =
          (import nixpkgs-oapi-gen { inherit system; }).openapi-generator-cli;

        # ---------------------------------------------------------------
        # Common worker runtimes (languages the worker executes)
        # ---------------------------------------------------------------

        commonRuntimes = with pkgs; [
          deno
          python3
          python3Packages.pip
          uv
          go
          bun
          nushell
          typescript
          flock
        ];

        # ---------------------------------------------------------------
        # Runtime PATH env vars — tells the worker where to find interpreters
        # ---------------------------------------------------------------

        commonRuntimeVars = {
          DENO_PATH = "${pkgs.deno}/bin/deno";
          GO_PATH = "${pkgs.go}/bin/go";
          BUN_PATH = "${pkgs.bun}/bin/bun";
          NODE_PATH = "${pkgs.nodejs}/bin/node";
          NODE_BIN_PATH = "${pkgs.nodejs}/bin/node";
          UV_PATH = "${pkgs.uv}/bin/uv";
          NU_PATH = "${pkgs.nushell}/bin/nu";
          FLOCK_PATH = "${pkgs.flock}/bin/flock";
          CARGO_PATH = "${rustStable}/bin/cargo";
          BASH_PATH = "bash";
          GIT_PATH = "${pkgs.git}/bin/git";
        };

        # ---------------------------------------------------------------
        # Extra language runtimes (full shell only)
        # ---------------------------------------------------------------

        coursier = pkgs.fetchFromGitHub {
          owner = "coursier";
          repo = "launchers";
          rev = "79d927f7586c09ca6d8cd01862adb0d9f9d88dff";
          hash = "sha256-8E0WtDFc7RcqmftDigMyy1xXUkjgL4X4kpf7h1GdE48=";
        };

        extraRuntimes = with pkgs; [
          dotnet-sdk_9
          php
          php84Packages.composer
          ruby_3_4
          jdk21
          ansible
          oracle-instantclient
        ];

        extraRuntimeVars = {
          JAVA_PATH = "${pkgs.jdk21}/bin/java";
          JAVAC_PATH = "${pkgs.jdk21}/bin/javac";
          COURSIER_PATH = "${coursier}/coursier";
          DOTNET_PATH = "${pkgs.dotnet-sdk_9}/bin/dotnet";
          DOTNET_ROOT = "${pkgs.dotnet-sdk_9}/share/dotnet";
          PHP_PATH = "${pkgs.php}/bin/php";
          COMPOSER_PATH = "${pkgs.php84Packages.composer}/bin/composer";
          RUBY_PATH = "${pkgs.ruby_3_4}/bin/ruby";
          RUBY_BUNDLE_PATH = "${pkgs.ruby_3_4}/bin/bundle";
          RUBY_GEM_PATH = "${pkgs.ruby_3_4}/bin/gem";
          ORACLE_LIB_DIR = "${pkgs.oracle-instantclient.lib}/lib";
          ANSIBLE_PLAYBOOK_PATH = "${pkgs.ansible}/bin/ansible-playbook";
          ANSIBLE_GALAXY_PATH = "${pkgs.ansible}/bin/ansible-galaxy";
          CARGO_SWEEP_PATH = "${pkgs.cargo-sweep}/bin/cargo-sweep";
        };

        # ---------------------------------------------------------------
        # General dev environment variables
        # ---------------------------------------------------------------

        devEnvVars = {
          DATABASE_URL = "postgres://postgres:changeme@127.0.0.1:5432/windmill?sslmode=disable";
          REMOTE = "http://127.0.0.1:8000";
          REMOTE_LSP = "http://127.0.0.1:3001";
          NODE_ENV = "development";
          NODE_OPTIONS = "--max-old-space-size=16384";
        };

        # ---------------------------------------------------------------
        # Helper scripts — base set (default + full)
        # ---------------------------------------------------------------

        helperScriptsBase = [
          (pkgs.writeScriptBin "wm" ''
            cd ./frontend
            npm install
            npm run ${if stdenv.isDarwin then "generate-backend-client-mac" else "generate-backend-client"}
            npm run dev "$@"
          '')
          (pkgs.writeScriptBin "wm-build" ''
            cd ./frontend
            npm install
            npm run ${if stdenv.isDarwin then "generate-backend-client-mac" else "generate-backend-client"}
            npm run build "$@"
          '')
          (pkgs.writeScriptBin "wm-migrate" ''
            cd ./backend
            sqlx migrate run
          '')
          (pkgs.writeScriptBin "wm-reset" ''
            sqlx database drop -f
            sqlx database create
            wm-migrate
          '')
          (pkgs.writeScriptBin "wm-minio" ''
            set -e
            cd ./backend
            mkdir -p .minio-data/wmill
            ${pkgs.minio}/bin/minio server ./.minio-data --console-address ":9001"
          '')
          (pkgs.writeScriptBin "wm-minio-keys" ''
            set -e
            cd ./backend
            ${pkgs.minio-client}/bin/mc alias set 'wmill-minio-dev' 'http://localhost:9000' 'minioadmin' 'minioadmin'
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

        # ---------------------------------------------------------------
        # Helper scripts — extra (full shell only)
        # ---------------------------------------------------------------

        helperScriptsFull = [
          (pkgs.writeScriptBin "wm-caddy" ''
            cd ./frontend
            xcaddy build "$@" \
              --with github.com/mholt/caddy-l4@145ec36251a44286f05a10d231d8bfb3a8192e09 \
              --with github.com/RussellLuo/caddy-ext/layer4@ab1e18cfe426012af351a68463937ae2e934a2a1
          '')
          (pkgs.writeScriptBin "wm-setup" ''
            sqlx database create
            wm-build
            wm-caddy
            wm-migrate
          '')
          (pkgs.writeScriptBin "wm-bench" ''
            deno run -A benchmarks/main.ts -e admin@windmill.dev -p changeme "$@"
          '')
        ];

        # ---------------------------------------------------------------
        # Shared inputs and settings for default + full shells
        # ---------------------------------------------------------------

        coreBuildInputs = nativeBuildDeps ++ commonRuntimes ++ [
          rustStable
          openapi-generator-cli
        ] ++ (with pkgs; [
          nodejs
          git
          sqlx-cli
          cargo-watch
          jq
          gnused

          # CLI tools (for AI agents and dev workflow)
          gh
          asciinema
          mermaid-cli
        ]);

        # Playwright: use Nix-provided browsers (version-matched to playwright-driver)
        # Mermaid/Puppeteer: point at Nix chromium (Puppeteer respects this env var)
        browserVars = {
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PUPPETEER_EXECUTABLE_PATH = "${pkgs.chromium}/bin/chromium";
          PUPPETEER_SKIP_DOWNLOAD = "true";
        };

        # Wrapper for the Nix-provided playwright CLI (version-matched to its browsers)
        playwrightWrapper = pkgs.writeShellScriptBin "playwright" ''
          export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
          exec ${pkgs.nodejs}/bin/node ${pkgs.playwright-driver}/cli.js "$@"
        '';

        # ---------------------------------------------------------------
        # sandbox-env script — outputs env vars for browser tooling
        # Usage: eval "$(sandbox-env)"
        # ---------------------------------------------------------------

        sandboxEnvScript = pkgs.writeShellScriptBin "sandbox-env" ''
          echo "export PLAYWRIGHT_BROWSERS_PATH=${pkgs.playwright-driver.browsers}"
          echo "export PUPPETEER_EXECUTABLE_PATH=${pkgs.chromium}/bin/chromium"
          echo "export PUPPETEER_SKIP_DOWNLOAD=true"
        '';

        # ---------------------------------------------------------------
        # pkg-config wrapper — bakes in the Nix pkg-config search path
        # so sandbox profiles (buildEnv) work without setting env vars.
        # ---------------------------------------------------------------

        pkgConfigWrapper = pkgs.writeShellScriptBin "pkg-config" ''
          export PKG_CONFIG_PATH="${pkgConfigPath}:$PKG_CONFIG_PATH"
          exec ${pkgs.pkg-config}/bin/pkg-config "$@"
        '';

        # ---------------------------------------------------------------
        # Installable sandbox profiles (nix profile install .#sandbox)
        # ---------------------------------------------------------------

        sandboxEnv = pkgs.buildEnv {
          name = "windmill-sandbox";
          paths = coreBuildInputs ++ helperScriptsBase
            ++ [ playwrightWrapper sandboxEnvScript pkgConfigWrapper pkgs.chromium ];
        };

        sandboxFullEnv = pkgs.buildEnv {
          name = "windmill-sandbox-full";
          paths = coreBuildInputs ++ extraRuntimes
            ++ helperScriptsBase ++ helperScriptsFull
            ++ [ playwrightWrapper sandboxEnvScript pkgConfigWrapper pkgs.chromium
                 pkgs.cargo-sweep pkgs.xcaddy pkgs.nsjail ];
        };

      in {

        # =============================================================
        # Installable profiles — for Docker / nix profile install
        # Usage: nix profile install .#sandbox
        # =============================================================

        packages.sandbox = sandboxEnv;
        packages.sandbox-full = sandboxFullEnv;
        packages.default = sandboxEnv;

        # =============================================================
        # default — daily driver for backend + frontend development
        # Usage: nix develop
        # =============================================================

        devShells.default = pkgs.mkShell (buildEnvVars // commonRuntimeVars // devEnvVars // browserVars // {
          buildInputs = coreBuildInputs;

          packages = helperScriptsBase ++ [ playwrightWrapper ];
        });

        # =============================================================
        # full — all language runtimes, k8s tooling, specialized scripts
        # Usage: nix develop .#full
        # =============================================================

        devShells.full = pkgs.mkShell (buildEnvVars // commonRuntimeVars // extraRuntimeVars // devEnvVars // browserVars // {
          buildInputs = coreBuildInputs ++ extraRuntimes ++ (with pkgs; [
            # Python extras
            poetry
            pyright
            openapi-python-client

            # LSP / editor
            svelte-language-server
            taplo

            # Extra dev tools
            cargo-sweep

            # Kubernetes
            minikube
            kubectl
            kubernetes-helm
            conntrack-tools
            cri-tools

            # Extra
            xcaddy
            nsjail
          ]);

          packages = helperScriptsBase ++ helperScriptsFull ++ [ playwrightWrapper ];
        });

        # =============================================================
        # wasm — WASM target compilation (nightly Rust)
        # Usage: nix develop .#wasm
        # =============================================================

        devShells.wasm = pkgs.mkShell (buildEnvVars // {
          hardeningDisable = [ "all" ];

          buildInputs = nativeBuildDeps ++ (with pkgs; [
            (rust-bin.nightly.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
              targets = [ "wasm32-unknown-unknown" "wasm32-unknown-emscripten" ];
            })
            wasm-pack
            deno
            emscripten
            nushell
            glibc_multi
          ]);
        });

        # =============================================================
        # cli — lightweight Bun-based CLI development
        # Usage: nix develop .#cli
        # =============================================================

        devShells.cli = pkgs.mkShell {
          shellHook = ''
            if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
              export FLAKE_ROOT="$(git rev-parse --show-toplevel)"
            else
              export FLAKE_ROOT="$PWD"
            fi
          '';

          buildInputs = with pkgs; [ bun nodejs git ];

          packages = [
            (pkgs.writeScriptBin "wm-cli" ''
              bun run $FLAKE_ROOT/cli/src/main.ts "$@"
            '')
            (pkgs.writeScriptBin "wm-cli-deps" ''
              pushd $FLAKE_ROOT/cli/
              ${if stdenv.isDarwin then
                "./gen_wm_client_mac.sh && ./windmill-utils-internal/gen_wm_client_mac.sh"
              else
                "./gen_wm_client.sh && ./windmill-utils-internal/gen_wm_client.sh"}
              popd
            '')
          ];
        };
      });
}
