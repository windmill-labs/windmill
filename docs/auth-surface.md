# Auth surface: facts that are easy to get wrong

Symbols, not line numbers, are cited: they drift less.

- **Credential precedence** (`windmill-api-auth/src/auth.rs` `extract_token`): `Authorization: Bearer`
  → `token` cookie → `?token=` query param. A URL with `?token=` is a credential on every route, but
  an existing cookie silently wins over it.
- **`AUTH_CACHE`** caches a token's identity for 120 s. Deleting a token row does not purge it: the
  DB trigger (`migrations/20260316000001_token_hash_pk_swap.up.sql`) notifies only for
  `label = 'session'` rows, and `delete_token` never calls `invalidate_token_from_cache`.
- **Sessions** are `token` rows with `label='session'` plus the HttpOnly `token` cookie, minted only
  by `create_session_token` (`windmill-api-users/src/users.rs`). `GET /api/users/refresh_token`
  mints one for any non-job token but returns plain text, no redirect.
- **`tokens/impersonate`** (superadmin) returns a multi-use token and sets no cookie.
- **`max_token_expiration_days`** caps `POST /users/tokens/create` and `tokens/impersonate`, by
  shortening the stored expiration (`cap_token_expiration`), never by refusing: the CLI
  authorization page, `wmill user create-token` and the editor's language-server token all pick a
  lifetime without reading the setting, and CLIs already installed never will. The CLI signs in
  again on its own when its token expires, which is why the authorization page labels it
  `cli-login:<username>`, reserved in `is_user_token` so its expiry does not email the user. A token
  owned by a service account is exempt: one in the workspace the token names, or in any workspace
  for a workspace-less token (for `tokens/impersonate`, the impersonated account). Any workspace
  admin can therefore create and impersonate a service account to hold an uncapped token, so the
  ceiling bounds personal tokens only. Only the stored expiration is capped: the auth lookup never
  reads the setting, so tokens that exist when it is turned on or lowered keep theirs, including
  none. Deliberately outside it: server-side mints (`create_token_internal` callers such as native
  trigger webhook tokens, which never expire for GitHub and Nextcloud), and tokens with their own
  fixed lifetime that outlive a short ceiling: sessions (`MAX_SESSION_VALIDITY_SECONDS`, 3 days, and
  re-mintable through `GET /users/refresh_token`) and MCP OAuth access tokens (7 days, with a
  rotating 30-day refresh token). Any logged-in user can read the setting through `GET
  /settings/global/{key}`, which the token form uses to offer only expirations within it. The
  settings API and config sync reject any value `parse_max_token_expiration_days` cannot read, since
  the token routes would read it as no ceiling; `parseMaxTokenExpirationDays` in the frontend must
  accept exactly the same values.
- **A token's label decides whether its expiry raises alerts.** When `delete_expired_items` removes
  an expired `token` row, the monitor emails the owner and raises a critical alert (if enabled);
  rows registered by `register_token_expiry_notification` also get an "expiring soon" warning first,
  except a token whose whole lifetime fits in `TOKEN_EXPIRY_WARNING_DAYS` (7), which gets no row
  since the warning would arrive minutes after it was created. Neither happens when `is_user_token`
  (`windmill-common/src/auth.rs`) reserves the label, so a token the system mints for itself,
  whether from the backend or from the frontend through `tokens/create`, needs a reserved label. An
  `ephemeral-` prefix needs no other change (keep it clear of `is_server_minted_label` if minted
  through `tokens/create`); a new prefix also goes into the SQL and Svelte mirrors that function's
  doc lists.
- **Every superadmin route refuses a job token**: `require_super_admin`
  (`windmill-api-auth/src/lib.rs`) errors on `authed.job_id.is_some()`. A script that needs
  `users/create`, `tokens/impersonate`, `set_login_type`, … must use a dedicated superadmin user
  token stored as a secret, never `$WM_TOKEN`. Token scopes cannot narrow superadmin routes.
- **A remote deploy token is a credential for another instance**, held per account and workspace
  (`remote_deploy_token`, encrypted under the workspace key, re-keyed by `set_encryption_key`).
  Its `email` references `password(email)` with `ON DELETE/UPDATE CASCADE`, so whatever deletes or
  renames an account takes the token along and a recycled address cannot inherit it; removal from
  a workspace deletes it explicitly (`delete_workspace_user_internal`, both `leave_workspace`). The
  row records the target it was granted for, and
  `remote_deploy::proxy` only sends it to a target still matching the workspace setting, so
  re-pointing the setting cannot redirect anyone's token to a URL of the admin's choosing.
  `require_own_credentials` refuses job, scoped and read-only tokens (on `set_target` too): the
  stored token carries none of their restrictions. The proxy turns the local session into a remote
  bearer credential, so every ambient-cookie vector becomes one on the remote: its URL carries the
  row's random `proxy_key` (a link riding the `SameSite=Lax` cookie cannot know it; a header would
  do, but the frontend's only per-call hook is the global `OpenAPI.HEADERS`, whose mere presence
  switches every download to in-memory blobs). The key is served `no-store`, withheld from the
  credentials `require_own_credentials` refuses, and masked in this instance's own request logs
  (`RedactedUri`, used by the request span and the log context); a reverse proxy in front still
  writes the full path to its access log. `connect` names the target the token was obtained for
  and is refused, before the token is sent anywhere, if the workspace now points elsewhere; the
  token only ever goes to that target. It then locks the account (`FOR KEY SHARE`), the membership
  (required unless the caller is a superadmin), the workspace key (the lock a rotation takes), and
  the target setting (the lock `set_target` writes under, re-checked there). So no removal,
  rotation or target change during its remote call leaves a row behind, under a stale key, or for
  a target no longer set. The membership is locked `NOWAIT`: account deletion takes the account
  before the membership and global offboarding the reverse, so waiting in either order can
  deadlock; a membership being changed refuses the connect with a retry message instead.
  Connecting by redirect: the remote's `/user/remote_deploy_authorize` page
  mints a token bound to the one remote workspace (`remote-deploy:<source host>`) only on an
  explicit Authorize, only for a callback whose path is `/remote_deploy/callback`, and refuses to
  render inside a frame; the token travels in the fragment, and the callback checks a single-use
  `state` the drawer stored, so no other page can plant a token as the user's. The proxy also refuses a path the URL parser would rewrite,
  serves every response under `CSP: sandbox` + `nosniff`, forwards only the method, query, body,
  content-type and accept — never this instance's cookie or token — and turns the target's 401
  into a 502, because the browser logs the user out of *this* instance on an unhandled 401.
- **`login_type`** (`password` table) is a free-form `VARCHAR(50)`. Password login and password
  reset require `login_type = 'password'`; `set_password` also accepts `pending_oauth` and turns
  the account into a `password` one in the same statement (an account created ahead of its owner
  gets its first credential that way, or through the OAuth claim below).
- **Login links** (`login_link` table, `POST /users/login_links` superadmin-only,
  `GET /auth/login_link/{token}` unauthenticated): single-use, ≤2 h, a session cookie and a
  302 to a same-origin `rd`. A link minted with `confirm` is the `/user/login_link` page
  instead, which spends it only on a click (`POST` to the same path, answering `{location}`), so
  a mail scanner opening it does not. `require_login_type` on the mint refuses (409) an account whose
  `login_type` has moved on — the way a caller re-entering an account it created stops being
  able to once the owner has a password or a provider.
- **Pre-approved trial offer** (`cloud_trial_offer`, cloud-only routes under
  `/users/cloud_trial_offer`): written by a superadmin at provisioning, consumed by
  `{consumed: true}` or by the portal's refusal; `…/go` is the one Windmill→portal hop that
  mints a portal login, over the same `CUSTOMER_SERVICE_TOKEN` trust the onboarding hook uses
  (`users_ee.rs`, the portal's admin token). It never expires on its own.
- **OAuth login** (`oauth2_ee.rs` `login_externally`, decision in `existing_login_decision`)
  matches an existing account by lowercased email only. Same provider → login; a
  `pending_oauth` account (see `PENDING_OAUTH_LOGIN_TYPE`) is **claimed** by the first login
  whose address the provider itself asserted and did not mark unverified — `login_type` becomes
  the client key and the hash is nulled; otherwise `require_preexisting_user_for_oauth` decides:
  on, *every* existing account is loggable-into by any provider; off, "exists but with a
  different login type". A new account gets `login_type = <client key>`.
- **OAuth email trust**: `LoginUserInfo.email_verified` is read leniently (bool or
  "true"/"false" strings) and is only consulted for the claim above; only GitHub is filtered to
  `primary && verified`; a missing email is fabricated from `name` as `<name>@windmill.dev` and
  reaches `login_externally` with `email_asserted = false`.
- **`GET /api/oauth/login/{client}`** is an unauthenticated 302 to the provider — a plain link
  from any page starts SSO.
- **`CLOUD_HOSTED`** is presence-tested (`windmill-common/src/worker.rs`): `CLOUD_HOSTED=false`
  still enables cloud mode. Of the routes above only the cloud trial offer and onboarding
  profile routes are cloud-gated; for the rest, cloud only adds quotas.
- **`CREATE_WORKSPACE_REQUIRE_SUPERADMIN`** defaults to `true` when unset; only the literal
  `"true"` enables it when set.
