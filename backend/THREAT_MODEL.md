# Threat Model: Windmill Backend

## 1. System context

Windmill is an open-source (AGPLv3) developer platform for internal tools,
workflows, background jobs, API integrations, and UIs — a self-hostable
alternative to Retool / Pipedream / Airplane. The backend is a Rust workspace
(~60 crates: `windmill-api`, `windmill-worker`, `windmill-queue`,
`windmill-common`, a family of `windmill-trigger-*` crates, `windmill-mcp`,
`windmill-sandbox`, etc.) fronting a PostgreSQL database. A Svelte 5 frontend
(not in scope here, but referenced where stored-XSS threats originate) is
served by the same instance. The product ships in a Community Edition (CE,
public Docker images) and an Enterprise Edition (EE, `*_ee.rs` files gated by
`enterprise`/`private`/`license` cargo features).

The defining characteristic for threat modeling is that **Windmill executes
arbitrary user-supplied code** (Python, TypeScript via Bun/Deno, Go, Bash,
SQL, GraphQL, PowerShell, Rust, …) on its workers, and **stores the
credentials to every system its users connect to** (databases, cloud
accounts, SaaS APIs, OAuth tokens). It is therefore simultaneously an
arbitrary-code-execution engine and a credential vault — compromising one
instance can pivot into an organization's entire connected estate. Crucially,
the owner confirms `nsjail` is **off by default everywhere** (`ENABLE_NSJAIL`
is opt-in) and network isolation (`clone_newnet`) is separately gated: the
*only* job isolation present in a default install is PID-namespace `unshare`.
Filesystem and outbound-network isolation are therefore absent unless an
operator deliberately enables them, which makes "weak-by-default isolation" a
more accurate frame than "sandbox escape" for typical deployments. Cross-tenant
separation is enforced in software via workspace IDs, token scopes, folder
ACLs, and Postgres row-level security; on the managed offering, sensitive
customers can opt into dedicated DB / worker / namespace infrastructure, but
the shared tier relies entirely on that software boundary. Administrators are 
strongly encouraged to use nsjail sandboxing and are reminded that if they don't, 
their security model is that they trust their developers that write code ran on windmill 
to not do anything TOO malicious on the workers. When the default
database secret backend is used, only per-workspace secret *variables* are
encrypted at rest — instance-level `global_settings` (OAuth client secrets,
SMTP, object-store keys, license) are stored plaintext, so a database read
yields the instance-wide credential set. Internet-facing instances are
typically exposed directly with no built-in rate limiting or WAF.

It is deployed self-hosted (Docker Compose, Kubernetes/Helm, bare metal), on
cloud providers, and as a Windmill-Labs-managed multi-tenant service. The API
server is internet-facing in most deployments; workers pull jobs from the
Postgres queue. The large public attack surface (a sprawling authenticated
HTTP API, unauthenticated public-app and webhook/trigger endpoints, outbound
HTTP from user code and proxies) combined with the high-value assets makes
authorization-enforcement bugs, SSRF, SQL injection, and sandbox escape the
dominant risk categories — a pattern strongly confirmed by the project's
published advisory history (73 GHSA advisories, several rated 9.9 critical).

## 2. Assets

| asset | description | sensitivity |
|---|---|---|
| Workspace encryption keys | Per-workspace key (`workspace_key`) used to encrypt secret variables (MagicCrypt256); decrypts all secrets in the workspace | critical |
| Secret variables | User secrets stored encrypted in `variable` (is_secret) | critical |
| Resource credentials | DB passwords, cloud creds, API keys, connection strings in `resource` JSONB | critical |
| OAuth / external-account tokens | Refresh/access tokens in `account`, MCP OAuth tables | critical |
| User password hashes | Argon2 hashes in `password` table | critical |
| API tokens & session cookies | Bearer tokens / cookies in `token`; superadmin & scoped tokens | critical |
| Instance global settings | License key, JWT secret, SUPERADMIN_SECRET, SMTP, object-store + secret-backend (Vault/KMS/SM) creds in `global_settings` | critical |
| Worker host & process integrity | The host that runs untrusted user code | critical |
| Cross-tenant / cross-workspace isolation | The software boundary separating workspaces, folders, and tenants | critical |
| Downstream connected systems | Windmill is a credential vault: stored creds reach external DBs, cloud accounts, SaaS | critical |
| Script / flow / app source | Customer IP & business logic in `script`, `flow`, `app`, `raw_app` | high |
| Job arguments, results & logs | `queue`/`completed_job` args+result, `job_logs`; routinely contain secrets | high |
| Object store / S3 data | Files uploaded/produced by jobs | high |
| Audit logs | `audit`/`audit_partitioned` action trail | high |
| Service availability | API server + worker fleet uptime | high |
| PII | User emails, group membership | medium |

## 3. Entry points & trust boundaries

| entry_point | description | trust_boundary | reachable_assets |
|---|---|---|---|
| EP1 Authenticated job-execution API | `jobs/run/preview`, `run/h/{hash}`, `run_flow/run_script` — runs user code on workers | authenticated user → arbitrary code on worker | Worker host, downstream systems, isolation, job args/results/logs |
| EP2 Unauthenticated public endpoints | `apps_u/*`, `jobs_u/getupdate*`, `scripts_u`, `settings_u`, `resources_u` (`public_app_layer.rs`) | unauth HTTP → app logic & job data | Job results, scripts, secrets, PII |
| EP3 HTTP-trigger & webhook ingestion | `/api/r/*`, GCP/Azure push, Slack callback, `capture_u/*` | untrusted webhook → job queue | Job execution integrity, worker host |
| EP4 Message-queue / native triggers | kafka, postgres, mqtt, websocket, nats, sqs, email triggers | external broker/message → job queue | Job execution integrity, availability |
| EP5 HTTP API authorization layer | Token/scope/RLS/folder-ACL enforcement across all workspaced routes (`windmill-api-auth`) | scoped token / low-priv user → other users' & workspaces' data | Scripts, job data, secrets, isolation |
| EP6 AI proxy & MCP endpoints | `ai/proxy/*`, `mcp` — resolve `$var:`/resources, proxy to LLM APIs, `X-Resource-Path` | authenticated user → outbound HTTP + secret resolution | Secrets, resource creds, internal network, downstream |
| EP7 Outbound HTTP from executors/resources | GraphQL/HTTP/Postgres executors, webhook delivery, `test_object_storage_config`, git clone, npm tarball fetch | user-controlled URL → server-side request | Cloud metadata, internal network, downstream creds |
| EP8 SQL query builders & contextual-var substitution | App DB query builder (`whereClause`/`tags`), Postgres-trigger `where_clause`, `%%WM_*%%` interpolation, `WM_INTERNAL_DB` | user input → raw SQL | Database, connected DBs |
| EP9 Worker sandbox | nsjail / unshare / dind / rootless podman isolating user code | user code → host & cross-tenant filesystem/network | Worker host, isolation, downstream |
| EP10 Worker code generation / wrappers | Entrypoint override, env-var names, workspace env interpolated into generated wrapper code | user-controlled identifier → executable code | Worker host, isolation |
| EP11 OAuth / OIDC / SAML / MCP-OAuth / logout | Login callbacks, MCP OAuth client registration, logout `rd` redirect | untrusted IdP / redirect input → session | Session tokens, accounts |
| EP12 Stored-content rendering | App builder HTML component, markdown, S3 download response headers | stored user content → admin browser (same origin) | Admin session, account takeover |
| EP13 Log/file reading & export endpoints | `service_logs`, `jobs_u/getupdate` log file read (symlinks), workspace/tarball export | authed/unauth request → arbitrary file or admin-only config | Arbitrary files, global settings |
| EP14 Secret-value & resource-value caches | In-memory caches in `windmill-store` keyed (historically un-keyed) by path | cache lookup crossing identity/folder boundary | Secret variables, resource creds |
| EP15 Deployment & runtime config | docker-compose defaults: dind, debugger (`REQUIRE_SIGNED_DEBUG_REQUESTS` now defaults to `true`; can still be overridden to `false`), CORS `Any`, default admin/`changeme`, exposed Postgres, `SUPERADMIN_SECRET`, `ENABLE_NSJAIL=false`, privileged containers | operator/infra default → full instance | All assets |
| EP16 Supply chain | Cached hub scripts, GitHub workflow actions, vendored deps, Docker base image | build/update-time input → host & build integrity | Worker host, build integrity |
| EP17 Token lifecycle | Token create/rescope/refresh, script-issued JWTs | scoped caller → broader privilege | Tokens, accounts, isolation |

## 4. Threats

| id | threat | actor | surface | asset | impact | likelihood | status | controls | evidence |
|---|---|---|---|---|---|---|---|---|---|
| T1 | SQL injection in app/internal query builders and trigger clauses compromises the metadata DB and connected databases | remote_auth | EP8 | Database, downstream connected systems | critical | almost_certain | partially_mitigated | sqlx parameterized queries elsewhere; query-builder safety reviews | GHSA-225c-j3xq-g6x6, GHSA-78p7-jc72-gv66, GHSA-hvc7-f67h-jx3g, GHSA-wrrg-f89m-f84q, GHSA-79vf-3qwm-2w64, GHSA-55p6-fxj4-v983, GHSA-5g4v-49rj-r52r, GHSA-x6cq-7xr8-53x3, 2cf4bb180b |
| T2 | Server-side request forgery via proxies/executors reaches cloud metadata, internal network, and downstream credentials | remote_auth | EP6, EP7 | Cloud metadata, internal network, downstream connected systems, resource creds | critical | almost_certain | partially_mitigated | SSRF URL validation + redirect-following disabled added piecemeal; MCP private URL access requires the instance-wide `ALLOW_PRIVATE_MCP_SERVER_URLS` opt-in; WebSocket trigger URLs (stored, test, and runnable-resolved) are SSRF-validated at connect time behind the `ALLOW_PRIVATE_WEBSOCKET_URLS` opt-in, and the trigger test route now requires `:write` scope; SAML IdP metadata URLs are SSRF-validated at load time behind the `ALLOW_PRIVATE_SAML_METADATA_URLS` opt-in; outbound network isolation (`clone_newnet`) is opt-in and off by default | GHSA-3ggp-h37f-5qfw, GHSA-98qq-g8rh-xhff, GHSA-hfw8-27mx-63jm, GHSA-3r59-qvvc-774j, GHSA-4pj9-w5jc-g8w7, GHSA-8hh3-jf25-78j5, GHSA-3pjm-4w7f-3r2w, GHSA-f44c-x9hq-h68r, GHSA-j4h4-f8fj-3m3c, 4b06881918, 96a8eb63d4, dbd3942ef3 |
| T3 | Broken authorization / IDOR lets a scoped token or low-privilege member read scripts, job data, and secrets across folders and workspaces | remote_auth | EP5, EP2, EP1 | Scripts, job data, secrets, isolation | critical | almost_certain | partially_mitigated | RLS, token scopes, folder ACLs, view-token HMAC (added incrementally); on managed, sensitive tenants can opt into dedicated DB/worker/namespace, but the shared tier IS the software boundary | GHSA-qfg7-x243-5hg4, GHSA-8x8x-88qc-qp4r, GHSA-2ppx-66jv-wpw5, GHSA-x3x7-g97v-mp59, GHSA-j276-g4h8-g6h5, GHSA-8mv7-hmrg-96xv, GHSA-x2wf-f962-7frq, GHSA-qc7c-gcw6-h4xp, GHSA-vxc5-w28p-m9xw, GHSA-2g34-wfvr-5qqj, GHSA-w7p6-wpxm-pp66, 7edf3f0212, 89a7a37776, ab11c7747a, 664edcdfb7 |
| T4 | Remote code execution by injecting attacker-controlled identifiers into generated worker wrappers | remote_auth | EP10 | Worker host, isolation, downstream | critical | likely | partially_mitigated | entrypoint/env-var-name validation added | GHSA-wxjq-w5pj-jqhx, GHSA-5f5q-2vg2-r2x4, GHSA-8q8j-mm3g-5c2q (CVE-2026-33881), bf93657fee, bd05bcadde, 22ec4da5f0 |
| T5 | Worker compromise & cross-tenant access via weak-by-default isolation (nsjail off by default → user code runs with only PID-ns `unshare`); sandbox escape where nsjail/dind/podman is enabled | remote_auth | EP9, EP15 | Worker host, isolation, downstream | critical | likely | unmitigated | nsjail off by default everywhere (`DISABLE_NSJAIL=true`); shipped compose gives PID-ns `unshare` only (`FAVOR_UNSHARE_PID=true`), bare installs get no isolation. Where nsjail enabled: read-only remounts, jail-tmp refusal, podman socket gating | GHSA-6qr8-xhg4-453q, GHSA-3vpp-vf62-wqp6, f8467f38c8, df5aec0f5d, f1b6746e0e |
| T6 | Disclosure of secrets, resource credentials, and workspace encryption keys across the authorization boundary (AI proxy, MCP, caches, export); database read additionally yields plaintext instance-level `global_settings` secrets | remote_auth | EP6, EP14, EP13 | Secret variables, encryption keys, resource creds, global settings | critical | likely | partially_mitigated | RLS on `$var:`, cache scoping by caller, admin checks on export; per-workspace secret *variables* encrypted at rest, but `global_settings` is plaintext under the default DB secret backend | GHSA-jwg4-v3cj-rvfm, GHSA-8m2p-2crh-9h3w, GHSA-6635-6fch-v8px, GHSA-437f-725p-7w84, GHSA-f27g-j463-q85w (CVE-2026-26964), GHSA-j679-v6vj-jfxc, GHSA-6vrr-fq33-qpfp, 0ba128afe7, 7836a4e733, ff8e39c69b |
| T7 | Full instance compromise from insecure deployment defaults (dind control, default admin/`changeme`, exposed Postgres, publicly readable SUPERADMIN_SECRET) | remote_unauth | EP15 | All assets | critical | likely | partially_mitigated | first-time-setup warning on default admin; docs recommend hardening | GHSA-3vpp-vf62-wqp6, GHSA-24fr-44f8-fqwg (CVE-2026-29059), GHSA-6q36-5p3h-766j |
| T8 | Unauthenticated RCE via the Debugger WebSocket: `/ws_debug/*` exposed by the gateway/ingress with the debugger service as the auth boundary; signature gate was bypassable via `program`-mode launches (read+exec an arbitrary server-side file path, never signed) even with signing on, and the WS handshake had no Origin check (CSWSH) | remote_unauth | EP15 | Worker host, all assets | critical | possible | partially_mitigated | `program`-mode launches now rejected when `REQUIRE_SIGNED_DEBUG_REQUESTS` is on (signing covers every launch, not just inline `code`); shipped `docker-compose` now defaults `REQUIRE_SIGNED_DEBUG_REQUESTS=true`; opt-in `DEBUG_ALLOWED_ORIGINS` allowlist rejects cross-origin handshakes. Residual: code default is secure but operators can still set `=false`; origin allowlist is opt-in | GHSA-725h-99vx-9xr4 |
| T9 | Supply-chain compromise via cached hub scripts, GitHub workflow command injection, or vulnerable base-image deps | supply_chain | EP16 | Worker host, build integrity | critical | possible | partially_mitigated | hub-script re-pin to patched versions; HUB_BASE_URL override | GHSA-w2m9-q5f7-3gpq, edf340c4d4, GHSA-8rq7-w7g6-8wvr, GHSA-vch9-39v5-4wg7 (CVE-2024-37371) |
| T10 | Unauthenticated disclosure of job results, args, logs, and admin config via missing-authz public endpoints | remote_unauth | EP2, EP13 | Job results/args/logs, global settings, scripts | high | likely | partially_mitigated | anonymous-job checks, log-endpoint authz hardening | GHSA-qfg7-x243-5hg4, GHSA-v448-fmm4-52fp, 108a88a180, bb90f4ce83 |
| T11 | Stored XSS leading to admin/account takeover via app HTML component, markdown, or S3 download content-type | remote_auth | EP12 | Admin session, accounts | high | likely | partially_mitigated | DOMPurify markdown sanitization, `X-Content-Type-Options: nosniff` + CSP sandbox on downloads | GHSA-9c5c-hh3c-r9mc, GHSA-qxj7-hpx3-r892, GHSA-cf2x-rg8c-v63v, bb78b1c06d, 625b67dff0 |
| T12 | Webhook authentication bypass / signature replay forges trigger invocations and approvals | remote_unauth | EP3 | Job execution integrity, approvals | high | likely | partially_mitigated | HMAC verification on some triggers; signing-oracle fix | GHSA-jw8c-h45c-xpjw, GHSA-hh9x-rcf8-xjr2, GHSA-q9g3-q6fj-hc2x, GHSA-8jc4-wj2p-2vmp, ab2a15b2a8 |
| T13 | Path traversal / arbitrary file read via log-reading and MCP path endpoints (incl. symlink following) | remote_auth | EP13 | Arbitrary files on server, global settings | high | likely | partially_mitigated | traversal checks + no-symlink-follow added | GHSA-4hrf-mgvv-xp9x, bb90f4ce83, df451aa64f, ad5ec293b5, 5f2d3e6812 |
| T14 | Privilege escalation via token rescope/refresh, script-issued JWTs, or operator-permission gaps | remote_auth | EP17, EP5 | Tokens, isolation, accounts | high | likely | partially_mitigated | monotonic-privilege enforcement on token lifecycle; SECURITY DEFINER triggers | GHSA-p62p-67xp-v775, GHSA-vv9w-wx3c-q3x2, 2ddf93de96, 865ab70c89, 33fb08cf3d |
| T15 | Credential leakage via worker `/proc` environment and unmasked secrets in job logs | remote_auth | EP9, EP1 | DB creds, secrets, downstream | high | likely | partially_mitigated | Aho-Corasick secret masking in logs | GHSA-pmp9-9924-f9cx, 0885d8c986 |
| T16 | Denial of service via resource exhaustion: unbounded uploads, runaway jobs, queue flooding, or trigger-message storms | remote_auth | EP1, EP3, EP4 | Service availability, worker fleet | high | likely | risk_accepted | Per-job rlimits/timeouts exist; instance-wide DoS by an authenticated tenant is largely accepted on shared self-host (operator's job to add global quotas). Hard requirement only for managed multi-tenant | |
| T17 | Account/credential theft via unauthenticated MCP-OAuth client registration and open redirect on logout | remote_unauth | EP11 | Accounts, session tokens | high | possible | partially_mitigated | redirect-URI handling / registration hardening | GHSA-q9xg-f2v2-695g, GHSA-53xj-pvqf-wpm9, GHSA-rr8j-ffc4-pf7h, GHSA-6c5w-777m-8rv5 |
| T18 | Account takeover via missing rate limiting / brute force on auth endpoints | remote_unauth | EP11 | Accounts | medium | likely | unmitigated | none built-in; owner confirms instances are typically exposed directly with no app-level rate limiting or WAF | GHSA-cmv6-m7wc-c87p |
| T19 | Enterprise license bypass and account impersonation | remote_auth | EP5 | Global settings, accounts | medium | possible | unmitigated | license validation gated by `license` feature | GHSA-48j5-p323-4mpx, GHSA-pv35-65rq-w29h, GHSA-2qx7-634r-qj6r |
| T20 | Trigger spoofing: an actor with broker/queue access injects messages that execute jobs without app-level auth | adjacent_network | EP4 | Job execution integrity, downstream | medium | possible | risk_accepted | Owner confirms trust is delegated to broker ACLs by design; no app-level message authenticity check. Anyone able to publish to a subscribed topic/queue can cause job execution | |
| T21 | Data-in-transit interception/tampering from TLS-disabled defaults (DB `sslmode=disable`, HTTP-only Caddy) | adjacent_network | EP15 | DB creds, secrets, session tokens | medium | possible | unmitigated | docs recommend TLS; not default | |
| T22 | Repudiation / incident blind spots from gaps in audit coverage of sensitive actions | remote_auth | EP5 | Audit logs | medium | possible | partially_mitigated | `windmill-audit` records many actions | |

## 5. Deprioritized

| threat | reason |
|---|---|
| Physical access to the host / cold-boot key extraction | Out of scope; deployment-environment responsibility, not addressable in this codebase |
| Memory-safety RCE in the Rust backend itself | Rust's safety model makes this rare; no evidence in history. Note: `unsafe` FFI (duckdb) is a narrow exception folded into supply-chain/T9 |
| Client-side-only nuisance bugs (CSS, layout) with no security impact | No asset compromised |
| Insider with legitimate superadmin / DB-root access | Trusted role; mitigations are operational (least privilege, audit), not technical controls in scope |
| Spoofing of a fully-trusted upstream IdP that has itself been compromised | Out of model; Windmill trusts the configured IdP by design |
| Instance-wide DoS by an authenticated tenant on shared self-host (T16) | Risk accepted (owner): per-job rlimits/timeouts are in place; global concurrency/queue quotas are the operator's responsibility on self-host. Remains a hard requirement for the managed multi-tenant fleet |
| Job execution triggered by an actor with legitimate broker/queue publish access (T20) | Risk accepted (owner): trigger authenticity is delegated to broker ACLs by design; consuming from a configured source and acting on its messages is the intended behavior |

## 6. Open questions

Facts that drove the score changes above. Two were confirmed in code during
the interview (`[Code-verified]`); the rest remain `[Owner-states]` pending a
check.

- [Code-verified] nsjail is off by default in every configuration: `DISABLE_NSJAIL` defaults to `true` (`windmill-worker/src/worker.rs:346`), and `is_sandboxing_enabled()` requires `DISABLE_NSJAIL=false` or the `job_isolation` global setting = `nsjail_sandboxing` (`worker.rs:890`). PID-ns `unshare` is also off at the code level (`is_unshare_enabled()`, `worker.rs:903`); the shipped `docker-compose.yml` sets `FAVOR_UNSHARE_PID=true` (line 91), so the official compose gives PID-ns unshare only, nsjail off — a bare install gets no isolation at all. No separate `clone_newnet` flag exists; network isolation is an nsjail feature, so outbound network from user code is unrestricted by default. Affects: T2 controls/likelihood, T5 status (unmitigated), T8.
- [Code-verified] `global_settings` is plaintext at rest under the default DB backend: `set_value_in_global_settings` stores the raw JSON value with no encryption (`windmill-common/src/global_settings.rs:259`); the encrypting secret backend (`secret_backend/database.rs:66`) only encrypts per-workspace `variable` rows with `is_secret=true`. Instance-level SMTP/OAuth/AI/object-store secrets are therefore plaintext. Affects: T6 impact/controls, T7.
- [Owner-states] Internet-facing instances are typically exposed directly with no built-in rate limiting / WAF. Affects: T16, T18 likelihood. Verify by: confirm absence of a rate-limit layer in `windmill-api/src/lib.rs` middleware stack.
- [Owner-states] Managed offering provides an optional dedicated DB/worker/namespace tier for sensitive tenants; the shared tier relies solely on the software authz boundary. Affects: T3 controls. Verify by: deployment topology (not in this repo) — out-of-tree.
- [Owner-states] Per-job rlimits/timeouts exist; instance-wide DoS by an authed tenant is risk-accepted on shared self-host. Affects: T16 status. Verify by: locate the rlimit/timeout enforcement in the worker execution path and confirm there is no global queue/concurrency cap.
- [Owner-states] Message-queue trigger authenticity is delegated to broker ACLs only. Affects: T20 status. Verify by: review `windmill-trigger-{kafka,sqs,nats,mqtt,postgres}` consume paths for any payload authentication.

## 7. Provenance

- mode: bootstrap-then-interview
- date: 2026-06-05
- target: /home/rfiszel/windmill/backend @ 819ba5e150
- inputs: git-log mined + GitHub security advisories (gh api, 73 advisories) + CHANGELOG; seed: THREAT_MODEL.md (bootstrap pass)
- owner: Ruben Fiszel (Windmill core dev)

## 8. Recommended mitigations

| mitigation | threat_ids | closes_class | effort |
|---|---|---|---|
| Centralize a single audited query-builder that forbids string-interpolated SQL; ban `format!`-built queries via lint/CI | T1 | yes | M |
| Route all outbound requests through one SSRF-guarded HTTP client (allowlist/denylist of private+metadata ranges, redirects disabled, re-validated per hop) | T2 | yes | M |
| Enforce authorization centrally in middleware (scope + RLS + folder ACL) with deny-by-default and a per-route coverage test, instead of per-handler checks | T3, T10, T14, T22 | yes | L |
| Treat all user-supplied identifiers as data: pass via argv/env/structured params, never splice into generated wrapper source; validate against strict allowlists at the boundary | T4 | yes | M |
| Make `nsjail` + network-namespace isolation default-on / fail-closed (flip `ENABLE_NSJAIL` and `clone_newnet` defaults) and remove privileged/dind defaults from shipped compose; default-deny debugger | T2, T5, T7, T8 | partial | L |
| Encrypt `global_settings` at rest under the workspace/instance key even on the default DB secret backend, so a DB read no longer yields plaintext instance-wide credentials | T6, T7 | partial | M |
| Ship hardened defaults: random per-install secrets, no default admin password, Postgres not exposed, CORS locked to configured origin, TLS-on | T7, T18, T21 | partial | M |
| Resolve secrets/resources only with the caller's identity and scope every cache entry by (caller, scope); apply uniformly to AI proxy, MCP, and exports | T6 | yes | M |
| Output-encode/sanitize all stored content at render and force `nosniff` + restrictive CSP on every user-content response | T11 | yes | M |
| Verify webhook authenticity uniformly (constant-time HMAC + timestamp/nonce anti-replay) in a shared trigger-auth helper | T12 | yes | S |
| Canonicalize + confine all file-path inputs to a base dir and never follow symlinks in log/file readers | T13 | yes | S |
| Mask secrets at the log sink and keep secrets out of worker process env (`/proc`) — pass via files/pipes scrubbed after use | T15 | partial | M |
| Add global rate limiting and per-tenant resource/queue quotas at the edge | T16, T18 | partial | M |
| Pin and integrity-verify hub scripts and CI actions; SBOM + automated base-image CVE scanning in release | T9 | partial | M |
