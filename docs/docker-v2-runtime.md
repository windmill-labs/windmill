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

1. **Pull/extract** (podman, rootless): `podman create --pull=<policy> <image>` +
   `podman export | tar -x` materializes the image's flattened root filesystem
   into `{job_dir}/rootfs`, and `podman inspect` reads its OCI config. podman's
   image store dedups pulls across jobs.
2. **Run** (the job's nsjail sandbox): nsjail binds each top-level entry of the
   rootfs in place (binding the whole rootfs at `/` trips nsjail's read-only
   remount of its base root in a rootless userns), mounts the standard
   pseudo-filesystems (`/proc` from the jail's pid namespace, a tmpfs `/tmp`,
   `/dev` nodes), maps uid/gid 0 inside → the worker user outside, and runs the
   command. The container *is* the jail.

```
# sandbox <image>  ─▶  podman create+export ─▶  {job_dir}/rootfs  ─▶  nsjail (chroot rootfs)
                       podman inspect (OCI config) ──────────────────▶  Env / Cmd / WorkingDir
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

- **Where pulls live:** podman's rootless graph root (default
  `$HOME/.local/share/containers/storage`) — persistent, dedups pulls across jobs.
  The per-job extracted rootfs lives in `{job_dir}/rootfs` and is removed with the
  job; the transient `rootfs.tar` is removed right after extraction.
- **Freshness (`SANDBOX_IMAGE_PULL_POLICY`, default `newer`):** `newer` re-pulls
  only when the registry digest changed (one cheap manifest check per job, no data
  transfer if unchanged) — so moving tags like `:latest` don't go stale. `missing`
  is fastest but tags can go stale; `always` re-checks every job. Pinning a digest
  (`img@sha256:…`) is immutable and never stale.
- **Per-image size cap (`SANDBOX_IMAGE_MAX_SIZE_MB`, default 0 = off):** images
  whose on-disk size exceeds the cap are rejected before extraction.
- **Cache size cap (`SANDBOX_IMAGE_CACHE_MAX_MB`, default 0 = off):** best-effort
  LRU eviction — after a run, the oldest images are removed until podman's image
  store is back under the cap. In-use images are never removed.

## Requirements

- `podman` (rootless) and `tar` on the worker for image pull/extract.
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

- Content-addressed rootfs cache keyed by image digest (today each job re-exports;
  podman's image store still dedups the network pull).
- Pre-pull size guard via `skopeo` manifest inspection (reject before download).
- Subuid-range nsjail variant for multi-uid images.
- Per-container isolated networking (slirp/pasta).
- Support under the non-nsjail `unshare` isolation mode.
