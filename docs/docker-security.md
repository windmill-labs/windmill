# Docker base-OS security patching

The runtime images are built on `debian:trixie-slim` (Debian stable). The base
runtime stages run `apt-get update && apt-get upgrade -y && apt-get install …`
so that base-OS packages pick up Debian security and point-release fixes at
build time, instead of staying frozen at whatever versions the base tag shipped.

## Where the upgrade lives

`apt-get upgrade -y` is applied in the first apt block of the three stages that
establish a runtime Debian layer:

- `Dockerfile` (the primary `windmill` / `windmill-ee` image)
- `docker/DockerfileSlim` (`windmill-slim`)
- `docker/DockerfileSlimEe` (`windmill-ee-slim`)

Every other runtime image inherits its base OS from one of these transitively,
so patching here is sufficient:

- `DockerfileFull`, `DockerfileFullEe`, `DockerfileCuda` build `FROM` the primary
  `windmill` / `windmill-ee` image.
- `DockerfileExtra` builds `FROM windmill-ee-slim`.

The nsjail *builder* stages are throwaway (only the compiled `nsjail` binary is
copied out), so they are intentionally not upgraded. `DockerfileMultiplayer`
(`node:slim`), the CLI, Caddy-L4, CUDA-only, and RHEL/dnf images are out of scope
for this apt-based patching.

## Why `apt-get upgrade` and not `unattended-upgrades` / pinning

Debian stable's archive only receives security updates and ABI-stable point
releases (e.g. `openssl 3.0.x → 3.0.x+deb12u2`, same soname). It does not ship
feature/major bumps, so a build-time `apt-get upgrade` cannot silently break a
pinned runtime dependency the way it might on a rolling distro. The default
`debian.sources` already includes the `*-security` suite, so a plain upgrade
picks up security fixes without extra machinery. `unattended-upgrades` adds a
package and config for no benefit in a build context (it does not run at build
time), and a pinned base digest would freeze the CVEs in place.

Note the runtime apt installs were already unpinned (only the throwaway nsjail
builder pins versions), so these images were never byte-for-byte reproducible in
this dimension; `upgrade` moves the same already-floating packages to their
patched versions rather than changing the reproducibility posture.

## Caching and freshness

`apt-get upgrade` sits in the same `RUN` as `apt-get update && install`. Docker
keys that layer on the command string plus the parent layer, not on package
contents, so an unchanged build is a cache hit and the upgrade does not re-run.
The layer is invalidated — and fresh patches are pulled — when the parent layer
changes, primarily when the mutable `debian:trixie-slim` base digest moves on a
Debian point release. That self-aligns: the cache refreshes when there is
something new to pick up.

Because of apt-cache staleness, a security fix that lands between base-digest
bumps will not be picked up by a cached build until the next bump. To close that
gap, rebuild and republish the `latest` / patch tags:

- on each Debian point release (base digest bump), and
- on a periodic cadence (e.g. per Windmill release), rebuilding the base stages
  with `--no-cache` if you need to force a fresh `apt-get upgrade` regardless of
  the base digest.

Scan the published images (e.g. Trivy / Defender) after rebuilds to confirm the
base-OS finding count stays low.

# Verifying image signatures, SBOMs and provenance

Release images are signed and attested at publish time:

- **cosign keyless signature** on the pushed manifest digest (index and
  per-arch manifests), via GitHub OIDC — no long-lived signing key exists
  (`.github/actions/sign-attest-image`).
- **SBOMs** are generated at build time (`sbom: true` on the depot build
  step) and embedded in the image index as BuildKit attestation manifests —
  one SPDX document per platform. They are part of the signed index digest,
  so the cosign signature covers them. They are not sent to a transparency
  log: SPDX documents for these images run tens of MB, beyond what Rekor or
  GitHub attestations accept as payloads.
- **SLSA build provenance** recorded as a GitHub artifact attestation and
  pushed to the registry (`actions/attest-build-provenance`).

## What is covered

Only images published from a release tag (`v*`) are signed: `windmill`,
`windmill-ee`, `windmill-ee-cuda`, `windmill-slim`, `windmill-ee-slim`,
`windmill-full`, `windmill-ee-full` (`.github/workflows/docker-image.yml`),
`windmill-cli` (`build_cli_image.yml`) and `windmill-extra`
(`publish_extra.yml`). The `:latest` and `:main` tags are repointed on
every `main` push as well as on releases, so they resolve to a signed
digest only until the next `main` build lands — verify a version tag or a
digest, not `:latest`. Development images (`:dev`, branch builds,
`windmill-test`), the dispatch-only RHEL/rpi images and the `caddy-l4`
image are not signed.

## How to verify

Signatures are keyless: trust is anchored in the Fulcio certificate identity,
which for these images is the *calling workflow file at a `v*` tag ref* in
this repository. Verify a signature with cosign (v2.x):

```bash
cosign verify \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/windmill-labs/windmill/\.github/workflows/(docker-image|publish_extra|build_cli_image)\.yml@refs/tags/v' \
  ghcr.io/windmill-labs/windmill:<version>
```

Extract the embedded SBOM (per platform; verify the signature first — it
covers the index these documents live in):

```bash
docker buildx imagetools inspect ghcr.io/windmill-labs/windmill:<version> \
  --format '{{ json .SBOM }}'
```

Verify SLSA provenance through GitHub's attestation API:

```bash
gh attestation verify oci://ghcr.io/windmill-labs/windmill:<version> \
  -R windmill-labs/windmill
```

Note for registry housekeeping: cosign stores signatures as extra
`sha256-<digest>.sig` tags in the same ghcr package, and the pushed
provenance attestations live there as referrer artifacts — any
tag-retention automation must not prune them.
