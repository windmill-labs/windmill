#!/usr/bin/env nu
let tag = $"dev-lsp-(git branch --show-current)";

# Build docker image and start server on localhost:3001
def main [
  --podman(-p) # Use podman
  # TODO:
  --detach(-d) # Run container in background and print container ID
  ] {
  if $podman {
    podman build --tag $tag .; podman run -p 3001:3001 $tag 
  } else {
    docker build --tag $tag .; docker run -p 3001:3001 $tag
  }
}

# Stop podman container
def "main stop" [] {
  try { docker stop (docker ps -q --filter $"ancestor=($tag)") }
  try { podman stop (podman ps -q --filter $"ancestor=($tag)") }
}
