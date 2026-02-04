# Backend Compilation Optimization

## Summary

Feature-gated heavy dependencies that were compiled by default but only used behind enterprise/EE feature flags. This reduces the default build from **761 crates to 609 crates** (20% reduction).

## Changes

### Root `Cargo.toml`
- Removed 12 unused direct dependencies: `kube`, `k8s-openapi`, `aws-sigv4`, `aws-sdk-config`, `opentelemetry-proto`, `systemstat`, `globset`, `libloading`, `bitflags`, `memchr`, `quote`, `pep440_rs`

### `windmill-worker/Cargo.toml`
- Made optional (only needed for EE OTEL tracing proxy): `hudsucker`, `hyper-http-proxy`, `hyper-tls`, `hyper-util`, `rcgen`, `opentelemetry-proto`, `prost`
- Made optional (only needed for EE features): `aws-config`, `aws-credential-types`, `aws-smithy-types`
- Created `otel_proxy` feature to group the OTEL proxy deps
- Updated `private` feature to include `otel_proxy`

### `windmill-common/Cargo.toml`
- Made optional: `aws-config`, `aws-credential-types`, `aws-smithy-types`, `systemstat`, `globset`
- Added AWS deps to `private`, `parquet`, `aws_auth`, `bedrock` features
- Added `systemstat` to `private` feature
- Added `globset` to `parquet` feature

### `windmill-api/Cargo.toml`
- Made optional: `aws-sigv4`, `aws-sdk-config`, `aws-credential-types`, `aws-smithy-types`, `windmill-parser-py-imports`, `windmill-autoscaling`
- Added AWS deps to `parquet` and `bedrock` features
- Added `windmill-parser-py-imports` to `python` and `agent_worker_server` features
- Added `windmill-autoscaling` to `enterprise` feature

### `windmill-autoscaling/Cargo.toml`
- Made optional: `kube`, `k8s-openapi` (only used in EE code)
- Added to `private` feature

### `parsers/windmill-parser-py-imports/Cargo.toml`
- Removed unused direct dependencies: `malachite`, `malachite-bigint` (still available transitively via `rustpython-parser`)

## Benchmarks

### Default build (no features)

| Metric | Before | After |
|---|---|---|
| Crates compiled | 761 | 609 |
| Notable deps eliminated | - | aws-sdk-config (9.5s), k8s-openapi (7.3s), zstd-sys (7.7s), kube-client (1.9s) |

### Incremental compilation (stable-state, warm cache)

| Scenario | Before | After |
|---|---|---|
| Touch `windmill-api/src/users.rs` | ~5.6s | ~5.4s |
| Touch `windmill-worker/src/worker.rs` | ~6.7s | ~6.2s |
| Touch `windmill-common/src/worker.rs` (cascade) | ~8.5s | ~8.5s |

Incremental compilation improvement from feature-gating alone is modest because the bottleneck is the compilation of the windmill crates themselves (especially windmill-api at 90k LOC), not the dependencies.

## Developer-Local Speed Tips

These settings are **not committed** because they are developer-local preferences that depend on toolchain availability. Combined, they yield ~16% faster incremental compilation.

### mold linker (~6% improvement)

Install `mold` and add to `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

### Reduced debug info (~10% improvement)

Add to `[profile.dev]` in `Cargo.toml`:

```toml
split-debuginfo = "unpacked"
debug = "line-tables-only"
```

### SQLX offline mode (~4% improvement)

If you're not modifying SQL queries:

```bash
export SQLX_OFFLINE=true
```

### Combined effect

| Scenario | Baseline | With all tips |
|---|---|---|
| Touch `windmill-api` file | 5.6s | **4.7s** |
| Touch `windmill-worker` file | 6.7s | **6.0s** |
| Touch `windmill-common` file (cascade) | 8.5s | **7.6s** |

## What would help more (future work)

The single biggest improvement would be **splitting `windmill-api`** (90k LOC) into smaller crates. Currently, any file change in the crate triggers re-analysis of all 90k lines. However, this requires significant refactoring due to tight coupling between the triggers subsystem, jobs, users, and the axum router initialization.
