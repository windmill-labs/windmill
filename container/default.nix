{ pkgs ? import <nixpkgs> { }, version ? builtins.getEnv "VERSION"
, imageName ? builtins.getEnv "IMAGE_NAME"
, projectRoot ? builtins.getEnv "PROJECT_ROOT", withGit ? true
, withKubectl ? true, withHelm ? true, withPowershell ? true }:

let
  versionFile = builtins.readFile "${projectRoot}/version.txt";
  actualVersion = if version != "" then
    version
  else
    builtins.substring 0 (builtins.stringLength versionFile - 1) versionFile;
  actualImageName = if imageName != "" then imageName else "windmill";

  buildInputs = with pkgs; [
    openssl
    openssl.dev
    libxml2.dev
    xmlsec.dev
    libxslt.dev
    pkg-config
    cmake
    clang
  ];

  PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig"
    (with pkgs; [ openssl.dev libxml2.dev xmlsec.dev libxslt.dev ]);

  windmill-frontend = pkgs.stdenv.mkDerivation {
    pname = "windmill-frontend";
    version = actualVersion;

    src = projectRoot;

    buildInputs = with pkgs; [ nodejs_20 ];

    buildPhase = ''
      export HOME=$(pwd)
      cd frontend
      npm config set strict-ssl false
      npm ci --verbose
      npm run generate-backend-client
      NODE_OPTIONS="--max-old-space-size=8192" npm run build
    '';

    installPhase = ''
      mkdir -p $out
      cp -r build $out/
    '';
  };

  windmill-binary = pkgs.rustPlatform.buildRustPackage {
    pname = "windmill";
    version = actualVersion;

    src = "${projectRoot}/backend";
    nativeBuildInputs = buildInputs;

    cargoLock = {
      lockFile = "${projectRoot}/backend/Cargo.lock";
      outputHashes = {
        "object_store-0.12.0" =
          "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
      };
    };

    buildFeatures = [
      "embedding"
      "parquet"
      "openidconnect"
      "jemalloc"
      "deno_core"
      "license"
      "http_trigger"
      "zip"
      "oauth2"
      "dind"
      "php"
      "mysql"
      "mssql"
      "bigquery"
      "websocket"
      "python"
      "smtp"
      "csharp"
      "rust"
      "static_frontend"
    ];
    doCheck = false;

    inherit PKG_CONFIG_PATH;
    SQLX_OFFLINE = true;
    FRONTEND_BUILD_DIR = "${windmill-frontend}/build";

    RUSTY_V8_ARCHIVE = let
      rustyV8Version = "130.0.2";
      target = pkgs.stdenv.hostPlatform.rust.rustcTarget;
      sha256 = {
        x86_64-linux = "sha256-ew2WZhdsHfffRQtif076AWAlFohwPo/RbmW/6D3LzkU=";
        x86_64-darwin = pkgs.lib.fakeHash;
        aarch64-darwin = "sha256-d1QTLt8gOUFxACes4oyIYgDF/srLOEk+5p5Oj1ECajQ=";
      }.${pkgs.stdenv.system};
    in pkgs.fetchurl {
      name = "librusty_v8-${rustyV8Version}";
      url =
        "https://github.com/denoland/rusty_v8/releases/download/v${rustyV8Version}/librusty_v8_release_${target}.a.gz";
      inherit sha256;
    };

    preBuild = ''
      mkdir -p frontend
      ln -s ${windmill-frontend}/build frontend/build
    '';
  };

  kubectl = pkgs.kubectl;
  helm = pkgs.kubernetes-helm;
  powershell = pkgs.powershell;

  wmill-cli = pkgs.stdenv.mkDerivation {
    pname = "wmill-cli";
    version = "latest";

    nativeBuildInputs = with pkgs; [ bun ];

    buildPhase = ''
      export HOME=$(mktemp -d)
      export BUN_INSTALL=$out
      bun install -g windmill-cli
    '';

    installPhase = ''
      mkdir -p $out/bin
      cp $BUN_INSTALL/bin/wmill $out/bin/ || cp $HOME/.bun/bin/wmill $out/bin/ || true
    '';
  };

  containerImage = pkgs.dockerTools.buildLayeredImage {
    name = actualImageName;
    tag = actualVersion;

    fromImage = null;

    contents = with pkgs;
      [
        windmill-binary
        pkgs.nsjail
        pkgs.claude-code
        wmill-cli
        cacert
        busybox
        bash
        tini

        python311
        python312
        uv

        nodejs_20

        go_1_25

        deno
        bun
        php83
        phpPackages.composer

        docker

        curl
        jq
        unzip
        unixODBC
        libxml2
        xmlsec
        openssl
        procps
        awscli2

        gcc
        gnumake
      ] ++ pkgs.lib.optionals withGit [ git ]
      ++ pkgs.lib.optionals withKubectl [ kubectl ]
      ++ pkgs.lib.optionals withHelm [ helm ]
      ++ pkgs.lib.optionals withPowershell [ powershell ];

    config = {
      Entrypoint =
        [ "${pkgs.tini}/bin/tini" "--" "${windmill-binary}/bin/windmill" ];
      ExposedPorts = { "8000/tcp" = { }; };
      Env = [
        "PATH=/usr/src/app:${windmill-binary}/bin:${wmill-cli}/bin:${pkgs.claude-code}/bin:${pkgs.busybox}/bin:${pkgs.python311}/bin:${pkgs.python312}/bin:${pkgs.nodejs_20}/bin:${pkgs.go_1_25}/bin:${pkgs.deno}/bin:${pkgs.bun}/bin:${pkgs.php83}/bin:${pkgs.phpPackages.composer}/bin:${pkgs.docker}/bin:${pkgs.curl}/bin:${pkgs.uv}/bin:${pkgs.gcc}/bin:${pkgs.gnumake}/bin"
        "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
        "NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
        "PYTHON_PATH=${pkgs.python312}/bin/python3"
        "NODE_PATH=${pkgs.nodejs_20}/bin/node"
        "GO_PATH=${pkgs.go_1_25}/bin/go"
        "DENO_PATH=${pkgs.deno}/bin/deno"
        "BUN_PATH=${pkgs.bun}/bin/bun"
        "UV_PATH=${pkgs.uv}/bin/uv"
        "DOCKER_PATH=${pkgs.docker}/bin/docker"
        "TZ=Etc/UTC"
        "UV_CACHE_DIR=/tmp/windmill/cache/uv"
        "UV_PYTHON_INSTALL_DIR=/tmp/windmill/cache/py_runtime"
        "UV_PYTHON_PREFERENCE=only-managed"
        "UV_TOOL_BIN_DIR=/usr/local/bin"
        "UV_TOOL_DIR=/usr/local/uv"
        "GOCACHE=/tmp/windmill/cache/go"
        "RUSTUP_HOME=/tmp/windmill/cache/rustup"
        "CARGO_HOME=/tmp/windmill/cache/cargo"
        "LD_LIBRARY_PATH=."
      ];
      WorkingDir = "/usr/src/app";
    };

    fakeRootCommands = ''
      mkdir -p /usr/src/app
      mkdir -p /usr/local/bin
      mkdir -p /usr/local/uv
      mkdir -p /tmp/windmill/cache/uv
      mkdir -p /tmp/windmill/cache/py_runtime
      mkdir -p /tmp/windmill/cache/go
      mkdir -p /tmp/windmill/cache/rustup
      mkdir -p /tmp/windmill/cache/cargo
      mkdir -p /tmp/windmill/logs
      mkdir -p /tmp/windmill/search
      mkdir -p /tmp/.cache
      chmod -R 777 /tmp/windmill
      chmod 777 /tmp/.cache
      ln -s ${windmill-binary}/bin/windmill /usr/local/bin/windmill
      ln -s ${wmill-cli}/bin/wmill /usr/local/bin/wmill
      ln -s ${pkgs.bun}/bin/bun /usr/bin/bun
      ln -s ${pkgs.bun}/bin/bun /usr/bin/node
    '';
  };

in { inherit windmill-binary windmill-frontend containerImage; }
