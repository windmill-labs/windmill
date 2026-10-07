{ pkgs ? import <nixpkgs> { config.allowUnfree = true; }
, version ? builtins.getEnv "VERSION", imageName ? builtins.getEnv "IMAGE_NAME"
, projectRoot ? builtins.getEnv "PROJECT_ROOT" }:

let
  versionFile = builtins.readFile "${projectRoot}/version.txt";
  actualVersion = if version != "" then
    version
  else
    builtins.substring 0 (builtins.stringLength versionFile - 1) versionFile;
  actualImageName = if imageName != "" then imageName else "windmill";

  windmill-binary = pkgs.stdenv.mkDerivation {
    pname = "windmill";
    version = actualVersion;

    src = pkgs.fetchurl {
      url =
        "https://github.com/windmill-labs/windmill/releases/download/v${actualVersion}/windmill-amd64";
      sha256 = "sha256-fl1At5xCKcqOshwg8W3EPTwS1nWF1+7kZ7McxxZklgU=";
    };

    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = [ pkgs.stdenv.cc.cc.lib pkgs.openssl.out pkgs.glibc ];

    dontUnpack = true;
    dontBuild = true;

    installPhase = ''
      mkdir -p $out/bin
      cp $src $out/bin/windmill
      chmod +x $out/bin/windmill
    '';
  };

  wmill-cli =
    pkgs.runCommand "wmill-cli" { nativeBuildInputs = [ pkgs.bun ]; } ''
      export HOME=$(mktemp -d)
      bun install -g windmill-cli
      mkdir -p $out/bin
      cp $HOME/.bun/bin/wmill $out/bin/
      chmod +x $out/bin/wmill
    '';

  containerImage = pkgs.dockerTools.buildLayeredImage {
    name = actualImageName;
    tag = actualVersion;

    contents = [
      windmill-binary
      wmill-cli
      pkgs.nsjail
      pkgs.claude-code
      pkgs.cacert
      pkgs.busybox
      pkgs.bash
      pkgs.nodejs_22
      pkgs.python311
      pkgs.uv
      pkgs.bun
      pkgs.docker
      pkgs.curl
      pkgs.jq
      pkgs.unzip
      pkgs.unixODBC
      pkgs.libxml2
      pkgs.xmlsec
      pkgs.openssl
      pkgs.git
      pkgs.wget
    ];

    config = {
      Entrypoint = [ "${windmill-binary}/bin/windmill" ];
      ExposedPorts = { "8000/tcp" = { }; };
      Env = [
        "PATH=${windmill-binary}/bin:${wmill-cli}/bin:${pkgs.claude-code}/bin:${pkgs.busybox}/bin:${pkgs.nodejs_22}/bin:${pkgs.python311}/bin:${pkgs.bun}/bin:${pkgs.docker}/bin:${pkgs.curl}/bin:${pkgs.uv}/bin:${pkgs.git}/bin:${pkgs.wget}/bin:${pkgs.jq}/bin"
        "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
        "TZ=Etc/UTC"
        "UV_CACHE_DIR=/tmp/windmill/cache/uv"
        "UV_PYTHON_INSTALL_DIR=/tmp/windmill/cache/py_runtime"
        "UV_PYTHON_PREFERENCE=only-managed"
        "UV_TOOL_BIN_DIR=/tmp/windmill/bin"
        "UV_TOOL_DIR=/tmp/windmill/uv"
      ];
    };

    fakeRootCommands = ''
      mkdir -p tmp/windmill/cache/uv
      mkdir -p tmp/windmill/cache/py_runtime
      mkdir -p tmp/windmill/bin
      mkdir -p tmp/windmill/uv
      mkdir -p tmp/windmill/logs
      mkdir -p tmp/windmill/search
      mkdir -p tmp/.cache
      chmod -R 777 tmp/windmill
      chmod 777 tmp/.cache
      mkdir -p usr/bin
      mkdir -p usr/src/app
      ln -s ${pkgs.claude-code}/bin/claude usr/bin/claude
      ln -s ${wmill-cli}/bin/wmill usr/bin/wmill
      ln -s ${windmill-binary}/bin/windmill usr/bin/windmill
      ln -s ${windmill-binary}/bin/windmill usr/src/app/windmill
      ln -s ${pkgs.bun}/bin/bun usr/bin/bun
      ln -s ${pkgs.nsjail}/bin/nsjail bin/nsjail
    '';
  };

in { inherit windmill-binary wmill-cli containerImage; }
