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
- **Every superadmin route refuses a job token**: `require_super_admin`
  (`windmill-api-auth/src/lib.rs`) errors on `authed.job_id.is_some()`. A script that needs
  `users/create`, `tokens/impersonate`, `set_login_type`, … must use a dedicated superadmin user
  token stored as a secret, never `$WM_TOKEN`. Token scopes cannot narrow superadmin routes.
- **`login_type`** (`password` table) is a free-form `VARCHAR(50)`. Password login, `set_password`
  and password reset all require `login_type = 'password'`.
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
  still enables cloud mode. None of the routes above are cloud-gated; cloud only adds quotas.
- **`CREATE_WORKSPACE_REQUIRE_SUPERADMIN`** defaults to `true` when unset; only the literal
  `"true"` enables it when set.
