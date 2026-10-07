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
copied out), so they are intentionally not upgraded. The CLI, Caddy-L4,
CUDA-only, and RHEL/dnf images are out of scope for this apt-based patching.

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

Published images are signed and attested at publish time:

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

Every workflow that publishes an image to ghcr signs it. The certificate
identity is the workflow file plus the git ref the run was started on, so it
differs per image family:

| Images | Workflow (`.github/workflows/`) | Trigger | Ref in the identity |
| --- | --- | --- | --- |
| `windmill`, `windmill-ee`, `windmill-ee-cuda`, `windmill-slim`, `windmill-ee-slim`, `windmill-full`, `windmill-ee-full` | `docker-image.yml` | `v*` tag push | `refs/tags/v<version>` |
| `windmill-extra` (release tags) | `publish_extra.yml` | `v*` tag push | `refs/tags/v<version>` |
| `windmill-cli` | `build_cli_image.yml` | `v*` tag push | `refs/tags/v<version>` |
| `windmill-extra` (custom tag) | `build-extra-image.yml` | manual dispatch | the dispatched ref |
| `windmill-ee-rhel9` | `build-publish-rh-image.yml` | manual dispatch | the dispatched ref |
| `windmill-ee-rhel8` (`-amd64` and `-arm64` tags) | `build-publish-rh8-image.yml` | manual dispatch | the dispatched ref |
| `windmill-rpi` | `docker-image-rpi4.yml` | manual dispatch | the dispatched ref |
| `caddy-l4` | `build-caddy-l4-image.yml` | `main` push, manual dispatch | `refs/heads/main`, or the dispatched ref |

A manually dispatched run is signed like any other, and its identity carries
the ref it was dispatched on: `refs/heads/main` for a build from `main`,
`refs/heads/<branch>` for a build from a branch, `refs/tags/v<version>` for a
build from a release tag. The workflow file that runs is the one at that ref,
so an identity ending in a branch other than `main` says the image was built
by that branch's copy of the workflow. Accept only the refs you trust.

The three release workflows sign on `v*` tag pushes only. Their `:latest`
and `:main` tags are repointed on every `main` push as well as on releases,
so they resolve to a signed digest only until the next `main` build lands:
verify a version tag or a digest, not `:latest`. Development images of the
main set (`:dev`, branch builds, `windmill-test`) are not signed.

A tag last pushed by a run that did not sign has no signature until the
workflow publishes it again.

## How to verify

Signatures are keyless: trust is anchored in the Fulcio certificate identity
described above. Verify a signature with cosign (v2.x), using the workflow
of the image's family:

```bash
# main set: docker-image.yml; windmill-extra: publish_extra.yml;
# windmill-cli: build_cli_image.yml
cosign verify \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github\.com/windmill-labs/windmill/\.github/workflows/docker-image\.yml@refs/tags/v' \
  ghcr.io/windmill-labs/windmill:<version>

# dispatch-built families, here RHEL9 built from main
cosign verify \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github\.com/windmill-labs/windmill/\.github/workflows/build-publish-rh-image\.yml@refs/(heads/main|tags/v)' \
  ghcr.io/windmill-labs/windmill-ee-rhel9:<tag>
```

One expression that accepts every family, built from `main` or from a
release tag, for a policy applied to all Windmill images at once:

```bash
cosign verify \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github\.com/windmill-labs/windmill/\.github/workflows/(docker-image|publish_extra|build_cli_image|build-extra-image|build-publish-rh-image|build-publish-rh8-image|docker-image-rpi4|build-caddy-l4-image)\.yml@refs/(heads/main|tags/v[0-9]+\.[0-9]+\.[0-9]+)$' \
  ghcr.io/windmill-labs/windmill-extra:<version>
```

It is looser than a per-family one: it does not tie an image name to the
workflow that should have produced it, and it accepts a `main` build where
a release is expected. Prefer the per-family expression when a policy covers
a single image.

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

`-R` alone accepts any workflow of this repository. To hold the provenance
to the same identity as the signature, name the workflow and the ref:

```bash
gh attestation verify oci://ghcr.io/windmill-labs/windmill-extra:<version> \
  -R windmill-labs/windmill \
  --signer-workflow windmill-labs/windmill/.github/workflows/publish_extra.yml \
  --source-ref refs/tags/v<version>
```

Note for registry housekeeping: cosign stores signatures as extra
`sha256-<digest>.sig` tags in the same ghcr package, and the pushed
provenance attestations live there as referrer artifacts — any
tag-retention automation must not prune them.
