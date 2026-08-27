# Auth surface: facts that are easy to get wrong

Verified 2026-08 by reading source. Line numbers drift; the symbols do not.

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
- **OAuth login** (`oauth2_ee.rs` `login_externally`) matches an existing account by lowercased
  email only and logs in **iff** `require_preexisting_user_for_oauth || login_type == client_name`;
  otherwise it rejects with "exists but with a different login type". It never rewrites
  `login_type`. A new account gets `login_type = <client key>`. With the setting on, *every*
  existing account becomes loggable-into by any provider (the `||` short-circuits).
- **OAuth email trust**: `LoginUserInfo` has no `email_verified`; only GitHub is filtered to
  `primary && verified`; a missing email is fabricated from `name` as `<name>@windmill.dev`.
- **`GET /api/oauth/login/{client}`** is an unauthenticated 302 to the provider — a plain link
  from any page starts SSO.
- **`CLOUD_HOSTED`** is presence-tested (`windmill-common/src/worker.rs`): `CLOUD_HOSTED=false`
  still enables cloud mode. None of the routes above are cloud-gated; cloud only adds quotas.
- **`CREATE_WORKSPACE_REQUIRE_SUPERADMIN`** defaults to `true` when unset; only the literal
  `"true"` enables it when set.
