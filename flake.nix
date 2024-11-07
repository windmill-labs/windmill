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
      buildInputs = with pkgs; [
        openssl.dev libxml2.dev xmlsec.dev libxslt.dev
        rust-bin.beta.latest.default nodejs go
        xcaddy sqlx-cli pkg-config
      ];
      environment = {
        NODE_ENV = "development";
        NODE_OPTIONS = "--max-old-space-size=16384";
        DATABASE_URL = "postgres://postgres:changeme@127.0.0.1:5432/";
        PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" (with pkgs; [
          openssl.dev
          libxml2.dev
          xmlsec.dev
          libxslt.dev
        ]);
      };
      rustcTarget = pkgs.hostPlatform.rust.rustcTarget;
      fetch_librusty_v8 = { version, shas }: pkgs.fetchurl {
        name = "librusty_v8-${version}";
        url = "https://github.com/denoland/rusty_v8/releases/download/v${version}/librusty_v8_release_${rustcTarget}.a.gz";
        sha256 = shas.${system};
      };
    in {
      devShell = pkgs.mkShell {
        inherit (environment) NODE_ENV NODE_OPTIONS DATABASE_URL PKG_CONFIG_PATH;
        buildInputs = buildInputs;
        packages = [
          (pkgs.writeScriptBin "wm-caddy" ''
            cd ./frontend
            xcaddy build $* \
              --with github.com/mholt/caddy-l4@145ec36251a44286f05a10d231d8bfb3a8192e09 \
              --with github.com/RussellLuo/caddy-ext/layer4@ab1e18cfe426012af351a68463937ae2e934a2a1
          '')
          (pkgs.writeScriptBin "wm-build" ''
            cd ./frontend
            echo $(pwd)
            npm install
            npm run ${if pkgs.stdenv.isDarwin then "generate-backend-client-mac" else "generate-backend-client"}
            npm run build $*
          '')
          (pkgs.writeScriptBin "wm-migrate" ''
            cd ./backend
            sqlx database reset
            sqlx database create
            sqlx migrate run
          '')
          (pkgs.writeScriptBin "wm-setup" ''
            wm-build
            wm-caddy
            wm-migrate
          '')
          (pkgs.writeScriptBin "wm" ''
            cd ./frontend
            npm run dev $*
          '')
        ];
      };
      packages.windmill-client = pkgs.stdenv.mkDerivation {
        pname = "windmill-client";
        version = (pkgs.lib.strings.trim (builtins.readFile ./version.txt));

        src = ./.;
        buildInputs = with pkgs; [ nodejs go ];

        buildPhase = ''
          export HOME=$(pwd)
          npm config set strict-ssl false
          cd frontend
          npm install --verbose
          npm run generate-backend-client-mac
          NODE_OPTIONS="--max-old-space-size=8192" npm run build
        '';

        installPhase = ''
          mkdir -p $out/build
          cp -r build $out/build
        '';
      };
      packages.windmill = pkgs.rustPlatform.buildRustPackage {
        inherit (environment) DATABASE_URL PKG_CONFIG_PATH;

        pname = "windmill";
        version = (pkgs.lib.strings.trim (builtins.readFile ./version.txt));

        src = ./.;
        nativeBuildInputs = buildInputs ++ [ self.packages.${system}.windmill-client ];

        cargoRoot = "backend";
        cargoLock = {
          lockFile = ./backend/Cargo.lock;
          outputHashes = {
            "php-parser-rs-0.1.3" = "sha256-ZeI3KgUPmtjlRfq6eAYveqt8Ay35gwj6B9iOQRjQa9A=";
            "progenitor-0.3.0" = "sha256-F6XRZFVIN6/HfcM8yI/PyNke45FL7jbcznIiqj22eIQ=";
            "rustpython-ast-0.3.1" = "sha256-q9N+z3F6YICQuUMp3a10OS792tCq0GiSSlkcaLxi3Gs=";
            "tiberius-0.12.2" = "sha256-s/S0K3hE+JNCrNVxoSCSs4myLHvukBYTwk2A5vZ7Ae8=";
            "tinyvector-0.1.0" = "sha256-NYGhofU4rh+2IAM+zwe04YQdXY8Aa4gTmn2V2HtzRfI=";
          };
        };

        buildFeatures = [ "enterprise" ];
        doCheck = false;
        preBuild = ''
          export HOME=$(pwd)
          npm config set strict-ssl false
          cd backend
          sqlx database create
          sqlx migrate run
        '';

        FRONTEND_BUILD_DIR = "${self.packages.${system}.windmill-client}/build";
        RUSTY_V8_ARCHIVE = (fetch_librusty_v8 {
          version = "130.0.1";
          shas = {
            x86_64-linux = pkgs.lib.fakeHash;
            aarch64-linux = pkgs.lib.fakeHash;
            x86_64-darwin = pkgs.lib.fakeHash;
            aarch64-darwin = "sha256-d1QTLt8gOUFxACes4oyIYgDF/srLOEk+5p5Oj1ECajQ=";
          };
        });
      };
    });
}
