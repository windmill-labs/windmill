#! /usr/bin/env nu
# ------------------------------------ #
# Build and Test docker images locally #
#                run:                  #
#        docker/dev.nu --help       #
# ------------------------------------ #
# NOTE:
# Does not work if built from git worktree branch

let branch = git branch --show-current;
# cp -f Caddyfile .dev-docker/Caddyfile

# --- Constraints --- #
def constraints [
  custom_dockerfile?: path
  features?: string
] {
    # if ($features | str contains "enterprise") {
    #   print "Building in EE mode"
    #   print "Is windmill ee private repo mounted? [Y/n]"; 
    #   let inp = input listen --types [key]

    #   if ($inp.code != "y") {
    #     # print "You can use --auto-substitute(-s) to automatically mount ee private repo (TODO)"
    #     print "Aborted"
    #     return;
    #   }
    # }
  if not ($features | str contains "static_frontend") {
    panic "If you compile without 'static_frontend' you won't be able to load frontend"
  }
  if ($features | str contains "enterprise") and not ($features | str contains "license") {
    panic "If you build enterprise you also need to include 'license' feature"
  } 
}
# --- Patches --- #
# ./Dockerfile
def patch_main_dockerfile [
  features?: string
  wasm_pkg?: string
] {
  let dockerfile = open ./Dockerfile
  # Spread wasm parser
  if $wasm_pkg != null {
    $dockerfile
      | str replace '# -- MACRO-SPREAD-WASM-PARSER-DEV-ONLY -- #' $"COPY ./backend/parsers/windmill-parser-wasm/pkg-($wasm_pkg) ./node_modules/windmill-parser-wasm-($wasm_pkg)"
      | return $in
  }

  print $dockerfile

  return $dockerfile
}

# ./docker/Dockerfile*
def patch_custom_dockerfile [
  custom_dockerfile: path
  features?: string
] {
  open $custom_dockerfile
    | str replace 'ghcr.io/windmill-labs/windmill-ee:dev' (get_ident -f $features)
    | str replace 'ghcr.io/windmill-labs/windmill:dev' (get_ident -f $features)
    | return $in
}

# ./docker-compose.yml
def patch_docker_compose [
  custom_dockerfile?: path
  features?: string
] {
  mut compose = open ./docker-compose.yml 

  $compose.services.windmill_worker.pull_policy = "never"
  $compose.services.windmill_worker_native.pull_policy = "never"
  $compose.services.windmill_server.pull_policy = "never"
  $compose.services.windmill_indexer.pull_policy = "never"

  if ($custom_dockerfile | default 'none' | path basename) == "DockerfileNsjail" {
    $compose.services.windmill_worker.privileged = true
    $compose.services.windmill_worker_native.privileged = true

    $compose.services.windmill_worker.environment ++= ["DISABLE_NSJAIL=false"]
    $compose.services.windmill_worker_native.environment ++= ["DISABLE_NSJAIL=false"]
  }
  return ($compose | to yaml)
}

# Build and Test docker images
#
# For more info run:
# docker.nu up --help
# docker.nu build --help
#
# Should be invoked in repo root
def main [] {
  ./docker/dev.nu --help;
}

# TODO:
# Also we need single tmpdir for it
def kill [] {}

# Get tag for dockerfile + features
def "main tag" [
  custom_dockerfile?: path  # Any dockerfile that depends on root Dockerfile
  --features(-f): string # Features that will be passed to base Dockerfile to compile windmill
] {
  get_ident $custom_dockerfile -f $features
}

# Run and Build (if not yet) docker image
def "main up" [
  custom_dockerfile?: path  # Any dockerfile that depends on root Dockerfile
  --features(-f): string = "static_frontend" # Features that will be passed to base Dockerfile to compile windmill
  --wasm-pkg: string # Include unreleased wasm parser in docker image
  --rebuild(-r) # Rebuild if there is existing image with same tag
  --yes(-y) # Say yes for every confirmation (TODO)
  --podman(-p) # Use podman as a backend (TODO)
] {
  constraints $custom_dockerfile $features

  # let base_ident = get_ident -f $features;
  let ident = get_ident $custom_dockerfile -f $features;

  if $rebuild or not (docker images | into string | str contains $ident) {
    let base_ident = get_ident -f $features;

    if $rebuild or not (docker images | into string | str contains $base_ident) {
      print $"Building base image with ident: ($base_ident)"
      docker build . -t ($base_ident) -f (wrap "Dockerfile" (patch_main_dockerfile $features $wasm_pkg)) --build-arg features=($features)
    }

    if $custom_dockerfile != null {
      # let ident = get_ident $custom_dockerfile -f $features;
      print $"Building image by path ($custom_dockerfile) with ident: ($ident)"
      docker build -f (wrap "DockerfileCustom" (patch_custom_dockerfile $custom_dockerfile $features)) -t $ident / 
    }
  } else {
    print $"($ident) was found"
  }

  print $"Running: ($ident)"
  WM_IMAGE=$"($ident)" docker compose -f (wrap "docker-compose.yml" (patch_docker_compose $custom_dockerfile $features)) up 
}

# TODO
def clone_ee_private [] {
  let rev = open backend/ee-repo-ref.txt
  git clone --depth 1 git@github.com:windmill-labs/windmill-ee-private.git .docker.nu/private-ee-repo-rev-($rev)
  # git checkout ddcfdfc18a9833a5fc4e62ad62a265ef1e06a0aa)
}

def get_ident [ custom_dockerfile?: path, --features(-f): string ] {
    let feat_postfix = if $features != null {
      $features |  split row ',' | each { |e| $e | str trim --left --right } | sort | str join "-";
    } else {
      "no-features"
    }

    if $custom_dockerfile != null {
      let df_postfix = $"($custom_dockerfile)" | split row "Dockerfile" | get 1 | str downcase;
      return ($branch | append $df_postfix | append $feat_postfix | str join "__");
    } else {
      return ($branch | append $feat_postfix | str join "-");
    }
}

def wrap [ filename content ] {
  let tmpfile = ".dev-docker-wrapper-" ++ $filename;
  $content | save -f $tmpfile 
  return $tmpfile
}

