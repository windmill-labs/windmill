{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src" # for rust-analyzer
          "rust-analyzer"
        ];
      };
      buildInputs = with pkgs; [
        openssl openssl.dev libxml2.dev xmlsec.dev libxslt.dev libtool
        rust nodejs
        postgresql
        pkg-config cmake
      ];
      PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" (with pkgs; [
        openssl.dev
        libxml2.dev
        xmlsec.dev
        libxslt.dev
      ]);
      RUSTY_V8_ARCHIVE =
        let
          version = "130.0.1";
          target = pkgs.hostPlatform.rust.rustcTarget;
          sha256 = {
            x86_64-linux = "sha256-qc25H3Aj2KRhsAZ+2SD1c4RmweVK07oW71opZXRuUoc=";
            aarch64-linux = "sha256-qc25H3Aj2KRhsAZ+2SD1c4RmweVK07oW71opZXRuUoc=";
            x86_64-darwin = pkgs.lib.fakeHash;
            aarch64-darwin = "sha256-d1QTLt8gOUFxACes4oyIYgDF/srLOEk+5p5Oj1ECajQ=";
          }.${system};
        in pkgs.fetchurl {
          name = "librusty_v8-${version}";
          url = "https://github.com/denoland/rusty_v8/releases/download/v${version}/librusty_v8_release_${target}.a.gz";
          inherit sha256;
        };
    in {
      devShell = pkgs.mkShell {
        buildInputs = buildInputs ++ (with pkgs; [
          git xcaddy sqlx-cli flock sccache
          deno python3 python3Packages.pip go bun uv
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
            npm run ${if pkgs.stdenv.isDarwin then "generate-backend-client-mac" else "generate-backend-client"}
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
            npm run dev $*
          '')
        ];

        inherit PKG_CONFIG_PATH RUSTY_V8_ARCHIVE;
        NODE_ENV = "development";
        NODE_OPTIONS = "--max-old-space-size=16384";
        DATABASE_URL = "postgres://postgres:changeme@127.0.0.1:5432/";
        REMOTE = "http://127.0.0.1:8000";
        REMOTE_LSP = "http://127.0.0.1:3001";
        RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        DENO_PATH = "${pkgs.deno}/bin/deno";
        PYTHON_PATH = "${pkgs.python3}/bin/python3";
        GO_PATH = "${pkgs.go}/bin/go";
        BUN_PATH = "${pkgs.bun}/bin/bun";
        UV_PATH = "${pkgs.uv}/bin/uv";
        FLOCK_PATH = "${pkgs.flock}/bin/flock";
        CARGO_PATH = "${rust}/bin/cargo";
      };
      packages.default = self.packages.${system}.windmill;
      packages.windmill-client = pkgs.buildNpmPackage {
        name = "windmill-client";
        version = (pkgs.lib.strings.trim (builtins.readFile ./version.txt));
      
        src = pkgs.nix-gitignore.gitignoreSource [] ./frontend;
        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [ nodejs pixman cairo pango ];
        doCheck = false;

        npmDepsHash = "sha256-NXk9mnf74+/k0i3goqU8Zi/jr5b/bmW+HWRLJCI2CX8=";
        npmBuild = "npm run build";

        postUnpack = ''
          mkdir -p ./backend/windmill-api/
          cp ${./backend/windmill-api/openapi.yaml} ./backend/windmill-api/openapi.yaml
          cp ${./openflow.openapi.yaml} ./openflow.openapi.yaml
        '';
        preBuild = ''
          npm run ${if pkgs.stdenv.isDarwin then "generate-backend-client-mac" else "generate-backend-client"}
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
            "php-parser-rs-0.1.3" = "sha256-ZeI3KgUPmtjlRfq6eAYveqt8Ay35gwj6B9iOQRjQa9A=";
            "progenitor-0.3.0" = "sha256-F6XRZFVIN6/HfcM8yI/PyNke45FL7jbcznIiqj22eIQ=";
            "tinyvector-0.1.0" = "sha256-NYGhofU4rh+2IAM+zwe04YQdXY8Aa4gTmn2V2HtzRfI=";
          };
        };

        buildFeatures = [
          "enterprise" "enterprise_saml" "stripe" "embedding" "parquet" "prometheus"
          "openidconnect" "cloud" "jemalloc" "tantivy" "deno_core" "license" "http_trigger"
          "zip" "oauth2" "kafka" "otel" "dind" "php" "mysql" "mssql" "bigquery" "websocket"
          "python" "smtp" "csharp" "static_frontend" "rust"
        ];
        doCheck = false;

        inherit PKG_CONFIG_PATH RUSTY_V8_ARCHIVE;
        SQLX_OFFLINE = true;
        FRONTEND_BUILD_DIR = "${self.packages.${system}.windmill-client}/build";
      };
    });
}
