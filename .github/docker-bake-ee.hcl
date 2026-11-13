# EE image and its debug file from one build: both targets share the builder
# stage, so the binary is compiled once and the .gnu_debuglink CRC in the shipped
# binary matches the uploaded debug file. Building them separately recompiles.

variable "WM_BUILD_VERSION" {}

variable "SBOM" {
  default = "false"
}

# Tags and labels, filled in by docker/metadata-action's bake files.
target "docker-metadata-action" {}

target "_ee" {
  context    = "."
  dockerfile = "Dockerfile"
  platforms  = ["linux/amd64", "linux/arm64"]
  args = {
    features         = "ee"
    WM_BUILD_VERSION = WM_BUILD_VERSION
  }
}

target "image" {
  inherits = ["_ee", "docker-metadata-action"]
  output   = ["type=registry"]
  attest   = SBOM == "true" ? ["type=sbom"] : []
}

# Writes ./debuginfo/linux_<arch>/windmill.debug. The file names every EE
# function with its source file and line: never push or publish it.
target "debuginfo" {
  inherits = ["_ee"]
  target   = "debuginfo"
  output   = ["type=local,dest=./debuginfo"]
}
