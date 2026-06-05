# Sandboxed container runtime (daemonless docker)

Windmill bash scripts can run a container image. There are **two** runtimes:

| | legacy `# docker` | sandboxed `# sandbox <image>` |
|---|---|---|
| selected by | bare `# docker` | `# sandbox <image>` |
| runtime | dind / Docker daemon (bollard, `dind` feature) | daemonless: extract rootfs + nsjail-run |
| boundary | separate (daemon outside the jail) | the job's own nsjail sandbox |
| nsjail | not provided (trusted-tenant) | **required** — this *is* the sandbox |
| safety | trusted-tenant | sandboxed (untrusted-capable) |
| compat | full `docker run`/`-d`/API | run-a-command subset |

The three bash annotations are distinct and don't overload each other:

- `# docker` → legacy daemon docker (unchanged).
- `# sandbox` → run the bash script under nsjail.
- `# sandbox <image>` → run that image's command under nsjail (this runtime).

## Using it

Put the image ref on a `# sandbox` annotation line; the rest of the script runs
**inside** that image:

```bash
# sandbox python:3.12-slim
name="$1"          # windmill args bind positionally, like any bash script
python3 -c "import sys; print('hello', sys.argv[1])" "$name"
```

- The body runs via the image's `/bin/sh -c` (so the image needs a shell).
- An **empty** body runs the image's `ENTRYPOINT` + `CMD`.
- Windmill args (declared `x="$1"`, …) are appended to the command.
- The image's `Env`, `WorkingDir` are applied; the windmill reserved variables
  (`WM_TOKEN`, `BASE_INTERNAL_URL`, …) are injected so `wmill`/API calls work.

## How it works

1. **Pull/extract** ([`crane`](https://github.com/google/go-containerregistry), no
   daemon/store/root): `crane export <image>` streams the image's flattened root
   filesystem to a tar (layers + whiteouts applied, like `docker export`) and
   `crane config` reads its OCI config. The tar + config are cached
   content-addressed by digest (`crane digest`) so unchanged digests reuse the
   cache; `tar -x` materializes the per-job `{job_dir}/rootfs`. crane is a single
   ~25 MB static binary — we never *run* the image with it (nsjail does), so a full
   container engine like podman isn't needed.
2. **Run** (the job's nsjail sandbox): nsjail binds each top-level entry of the
   rootfs in place (binding the whole rootfs at `/` trips nsjail's read-only
   remount of its base root in a rootless userns), mounts the standard
   pseudo-filesystems (`/proc` from the jail's pid namespace, a tmpfs `/tmp`,
   `/dev` nodes), maps uid/gid 0 inside → the worker user outside, and runs the
   command. The container *is* the jail.

```
# sandbox <image>  ─▶  crane export → digest-keyed rootfs cache → tar -x → {job_dir}/rootfs  ─▶  nsjail (chroot rootfs)
                       crane config (OCI config) ───────────────────────────────────────────▶  Env / Cmd / WorkingDir
```

Because the run is just the job's own nsjail with the image's filesystem as root,
the container inherits exactly the job's confinement:

- **Filesystem**: only the rootfs + the job's mounts are visible — no host `/`,
  no other job dirs, no dep cache. There is nothing to bind-mount escape to.
- **/proc**: the jail's own pid namespace — the worker and other jobs aren't
  visible.
- **uid**: a single-uid jail — an escape lands as the unprivileged worker user.
- **network**: the job's network (same as any bash job).

## Image storage, freshness & limits

- **Where pulls live:** a content-addressed cache of flattened rootfs tars (+ OCI
  config sidecars) keyed by image digest, under `{ROOT_CACHE_DIR}/sandbox_rootfs`
  (persistent, dedups pulls across jobs). The per-job extracted rootfs lives in
  `{job_dir}/rootfs` and is removed with the job.
- **Freshness (`SANDBOX_IMAGE_PULL_POLICY`, default `newer`):** the cache is keyed
  by digest, so a moving tag whose digest changed re-pulls automatically. `newer`
  (default) / `always` re-resolve the digest each job (one cheap `crane digest`
  manifest fetch); `missing` reuses a cached digest for the ref without hitting the
  registry; `never` only uses the cache (errors if absent). Pinning a digest
  (`img@sha256:…`) is immutable and never stale.
- **Per-image size cap (`SANDBOX_IMAGE_MAX_SIZE_MB`, default 0 = off):** images
  whose *compressed download* size (`crane manifest`) exceeds the cap are rejected
  **before any layer is downloaded**.
- **Cache size cap (`SANDBOX_IMAGE_CACHE_MAX_MB`, default 0 = off):** best-effort
  eviction — after a run, the oldest cached rootfs tars (by creation time) are
  removed until the cache is back under the cap.

## Requirements

- [`crane`](https://github.com/google/go-containerregistry) and `tar` on the worker
  for image pull/extract (a single static binary — no daemon, root, or privileged).
- `nsjail` on the worker — **required**. If nsjail is absent, a `# sandbox <image>`
  job errors clearly (use a bare `# docker` + a daemon instead).

## Limitations (by design — daemonless, run-to-completion)

- No `docker run -d` + later `exec`/`attach`/`logs -f`, no `docker build`,
  `compose`, swarm, healthchecks.
- No arbitrary `-v` host bind mounts, `--privileged`, `--cap-add`, `--device`,
  host namespace sharing.
- Images that drop to a non-root uid or chown to arbitrary uids inside need a
  subuid **range** in the jail (single-uid only today — follow-up: `newuidmap`
  range mapping).
- The script result is a completion message; capture output via stdout/logs.

## Follow-ups

- Subuid-range nsjail variant for multi-uid images.
- Per-container isolated networking (slirp/pasta).
- Support under the non-nsjail `unshare` isolation mode.
