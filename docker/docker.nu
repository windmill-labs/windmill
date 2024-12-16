#! /usr/bin/env nu
# ------------------------------------ #
# Build and Test docker images locally #
#                run:                  #
#        docker/docker.nu --help       #
# ------------------------------------ #

let branch = git branch --show-current;
# TODO: Use podman
let use_podman = false;

# Build and Test docker images
#
# For more info run:
# docker.nu up --help
# docker.nu build --help
#
# Should be invoked in repo root
def main [] {
  ./docker/docker.nu --help;
}

# Run and Build (if not yet) docker image
def "main up" [
  custom_dockerfile?: path  # Any dockerfile that depends on root Dockerfile
  --features(-f): string = "deno_core" # Features that will be passed to base Dockerfile to compile windmill
  --dir(-d): path = ../windmill-ee-private # Path to EE repo
] {
  let ident = get_ident $custom_dockerfile -f $features;
  if not (sudo docker images | into string | str contains $ident) {
    print $"($ident) is not found, building..."
    main build $custom_dockerfile -f $features -d $dir
  } else {
    print $"($ident) was found"
  }
  print $"Running: ($ident)"
  sudo WM_IMAGE=($ident) docker compose up --pull never
}

# Build docker image
def "main build" [
  custom_dockerfile?: path  # Any dockerfile that depends on root Dockerfile
  --features(-f): string = "deno_core" # Features that will be passed to base Dockerfile to compile windmill
  --mock(-m) # Ignore building base image
  --dir(-d): path = ../windmill-ee-private # Path to EE repo
] {
  if not $mock {
    build_base -f $features -d $dir;
  }
  if $custom_dockerfile != null {
    let ident = get_ident $custom_dockerfile -f $features;
    let tmpfile = patch $custom_dockerfile -f $features;
    print $"Building image by path ($custom_dockerfile) with ident: ($ident)"
    sudo docker build -f $tmpfile -t $ident ../
  }
}

def build_base [
  --features(-f): string
  --dir(-d): path
] {
  if not ($features | str contains "deno_core") {
    print "Warning! deno_core feature not enabled"
    print "Compilation may fail"
  }

  if ($features | str contains "enterprise") {
    print "Building in EE mode"
    print $"EE repo path: ($dir)"
    (cd ./backend; ./substitute_ee_code.sh --copy --dir $dir)
  }

  let ident = get_ident -f $features;
  print $"Building base image with ident: ($ident)"
  sudo docker build -t ($ident) ../ --build-arg features=($features)
  (cd ./backend; ./substitute_ee_code.sh --revert --dir $dir )
}

def get_ident [
  custom_dockerfile?: path  # Any dockerfile that depends on root Dockerfile
  --features(-f): string = "deno_core" # Features that will be passed to base Dockerfile to compile windmill
] {
    let feat_postfix = $features |  split row ',' | each { |e| $e | str trim --left --right } | sort | str join "-";
    if $custom_dockerfile != null {
      let df_postfix = $"($custom_dockerfile)" | split row "Dockerfile" | get 1 | str downcase;
      return ($branch | append $df_postfix | append $feat_postfix | str join "__");
    } else {
      return ($branch | append $feat_postfix | str join "-");
    }
}

def patch [
  file: path
  --features(-f): string
] {
  let tmpfile = $"(mktemp -d)/Dockerfile";
  open $file | str replace 'ghcr.io/windmill-labs/windmill-ee:dev' (get_ident -f $features) | save $tmpfile;
  return $tmpfile;
}
