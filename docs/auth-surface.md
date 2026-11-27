# Auth surface: facts that are easy to get wrong

Symbols, not line numbers, are cited: they drift less.

- **Credential precedence** (`windmill-api-auth/src/auth.rs` `extract_token`): `Authorization: Bearer`
  → `token` cookie → `?token=` query param. A URL with `?token=` is a credential on every route, but
  an existing cookie silently wins over it.
- **`AUTH_CACHE`** caches a token's identity for 120 s. Deleting a token row purges it only for
  some rows: the delete trigger (`notify_token_invalidation`, last replaced in
  `migrations/20261106141303_mcp_oauth_token_delete_revokes_grant.up.sql`) notifies every
  instance for `session` and `mcp-oauth-*` rows, within the notify poll (≤10 s), and
  `delete_token` also evicts at once on the instance serving it. Other labels deleted by other
  paths stay cached until the TTL, which ignores the token's own expiration.
- **Deleting a live MCP OAuth access token revokes its refresh grant**, by whatever path: the same
  trigger marks the `mcp_oauth_refresh_token` row issued with it (`access_token_hash`) revoked, so
  only that client's connection ends. A token past its expiration keeps its grant, since the
  expiry sweep deletes it and its client must still refresh. A client idle past its access
  token's 7 days therefore has no row left to delete: what ends a user's access calls
  `revoke_mcp_oauth_grants` (`windmill-api-users`), which also deletes grants without a live
  token and codes not yet exchanged — a password change or reset, disabling or deleting the
  account, leaving the instance, offboarding, SCIM deactivation and removal, and (for the
  grants bound to that workspace) removal from a workspace. The grant lock
  (`lock_mcp_oauth_grants`) is the user's `password` row, shared while issuing a grant and exclusive while revoking, because every path
  that changes a user locks that row before their tokens: a refresh takes it, then its
  access-token row, then its grant, the order the trigger also follows. A new path that writes a
  user's tokens and may revoke must lock that row first too. `leave_workspace` deletes no tokens
  and revokes nothing; `leave_instance` deletes every token of the account, as `delete_user` does.
- **Two-factor (TOTP) guards `POST /auth/login` only** (`windmill-api-users/src/totp_ee.rs`,
  with a stub in `totp_oss.rs`; a build without `enterprise` allows it for one user). The
  password is checked first, so "Two-factor code required" tells only someone who knows the
  password that a code is needed. Every other way into a session skips it on purpose: OAuth and
  SAML (the provider owns the second factor), login links (minted by a superadmin, who can reset
  the second factor anyway, and never for a superadmin or devops account), and `GET /users/refresh_token` (a session already exists).
  Authenticator codes are single-use (`totp_last_step`), and so are the ten recovery codes, stored
  as salted argon2 hashes (a code carries about 52 bits, too few for a plain digest) and are
  accepted wherever a code is. Wrong codes are capped per account at 5 a minute on the
  `password` row, so the cap holds across servers; the caller commits the count even when it
  refuses the code. Without that cap, a known password leaves a 6-digit code as the only barrier.
  Enrolling (`setup`) asks for the password again, so a stolen session cannot add its own
  authenticator and lock the owner out of password login. A superadmin removes a lost second
  factor with `POST /users/totp/reset/{user}`. The secret is
  encrypted under the `admins` workspace key, and rotating that key re-encrypts it
  (`set_encryption_key`); a secret left under an old key locks its user out. The global user
  export leaves it out, as its files end up in git: an overwrite keeps the second factor of every
  account it reinstates, and a push to another instance arrives without one.
  The `require_totp` instance setting (enterprise only, `totp::required`) makes a password login
  without a second factor answer "Two-factor setup required" once the password matched. The
  account then enrolls before it has any session, through `POST /auth/login/totp/setup` and
  `/auth/login/totp/enable`, which take the password again on each call; `enable` turns the
  factor on and mints the session in one transaction, so no session exists without it. While
  required, `disable` is refused; a superadmin `reset` is not, and sends the user back through
  enrollment. Sessions minted before the setting was turned on stay valid until they expire,
  but `refresh_token` no longer renews one for an account without a second factor, so it ends
  within `MAX_SESSION_VALIDITY_SECONDS` and the next login goes through the setup. The app
  warns in its last 30 minutes, and enrolling from the settings restores renewal.
  Until an account enrolls, its password is its only factor: whoever logs in with it first
  enrolls their own authenticator, so a password already stolen is not protected, and the owner
  needs a superadmin `reset` to recover. Turning the factor on (in settings or at login), off,
  regenerating its recovery codes and a superadmin `reset` each email the account's owner when
  SMTP is set up (`totp_ee::notify`), after the change has committed, so a change made with a
  stolen password or session reaches the owner.
- **Sessions** are `token` rows with `label='session'` plus the HttpOnly `token` cookie, minted only
  by `create_session_token` (`windmill-api-users/src/users.rs`). `GET /api/users/refresh_token`
  mints one for any non-job token but returns plain text, no redirect.
- **The readable `wm_session` cookie** is a hint of the session (a salted hash of the token, so it
  grants nothing), mirrored onto every response that sets or clears `token`, by middleware
  (`windmill-api/src/session_hint.rs`) rather than at each cookie site, so a new site gets it
  for free. The frontend renders a reload as the user `whoami` last confirmed only while the
  hint still matches the one stored with them (`frontend/src/lib/lastSession.ts`): logout,
  expiry or another login changes or drops it. A session revoked server-side keeps its hint,
  so until `whoami` answers that browser still renders as its own last user.
- **`tokens/impersonate`** (superadmin) returns a multi-use token and sets no cookie.
- **Token risk flags** (rights the token was minted with and no scope narrows, not bound to a
  workspace, no expiration, unused 90 days) are computed once, by the `token_risk` view, which also
  marks the tokens the server expires or renews on its own (`self_managed`, no flags). The user's
  `tokens/list` and `tokens/risk_counts` and the superadmin review read it: its counts
  (`tokens/risk_counts_as_super_admin`) in CE, its list (`tokens/list_as_super_admin`, cursor-paged
  on `(last_used_at, token_prefix)`) and `tokens/delete_as_super_admin/{email}/{prefix}` in EE
  (`users_ee.rs`); `frontend/src/lib/components/settings/tokenRisks.ts` only words the flags.
- **`GET /api/tokens/introspect`** describes the credential presenting it, never one named in
  the request, and resolves it in the handler rather than through `ApiAuthed`: the route layer
  would refuse a scoped token there, and a workspace-bound one resolves on no global route. Its
  route verdict is `check_route_layer` (`windmill-api-auth/src/auth.rs`), the function the auth
  extractor runs, plus the runnable scope of the run-by-path endpoints (`run_by_path_scope`); a
  handler that checks a scope of its own and is missing there gets an over-reported "allowed".
- **`max_token_expiration_days`** caps `POST /users/tokens/create` and `tokens/impersonate`, by
  shortening the stored expiration (`cap_token_expiration`), never by refusing: `wmill user
  create-token` and the editor's language-server token pick a lifetime without reading the
  setting, and CLIs already installed never will. The CLI authorization page (`/user/cli`) reads
  it only to offer the lifetimes that would be kept (30 days by default). The CLI signs in again
  on its own when its token expires, which is why the authorization page labels it
  `cli-login:<username>`, reserved in `is_user_token` so its expiry does not email the user.
  `wmill workspace login` replaces a profile's token before it expires, and revokes the previous
  one only when that label shows the browser login minted it. A token
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
- **A token carries its owner's superadmin and workspace admin rights only when minted with
  them** (`token.super_admin`, `token.admin`). `super_admin` lets a token into a workspace its
  owner is not a member of and makes a workspace-less MCP token list every workspace; on any row
  but a session it also decides the superadmin routes, which otherwise key off the account's
  email: the auth lookup sets `ApiAuthed::super_admin_withheld`, read by `require_super_admin`,
  `is_super_admin_authed` and `require_devops_role`, and `admin = false` caps `is_admin` there
  (`admin_withheld`, also read by the cross-workspace `usr.is_admin` checks). `POST
  /users/tokens/create` and `tokens/impersonate` withhold both unless the request sets them;
  `create_token_internal` called directly (server-side mints) inherits them from the owner,
  except for an MCP token's superadmin. Either way a token never gets rights the minting
  credential withholds, and `GET /users/refresh_token` refuses one that withholds rights the
  account holds, since a session carries them all. A job started by such a token runs with its
  capped `is_admin` (and a `$WM_TOKEN` cannot reach `tokens/create`); but what the token deploys
  or schedules runs later as its owner (`fetch_authed_from_permissioned_as`), with every right of
  the account, as with scoped tokens. Losing superadmin follows the owner without
  help from the caller: the deferred trigger `password_superadmin_demote_tokens` clears the column
  on every token of an address that, at commit, no longer has a superadmin `password` row
  (demotion, deletion, an import's delete-and-reinsert), and evicts each through
  `notify_token_invalidation`. An email change does not fire it, since SCIM's rename leaves the
  tokens under the old address. A promotion grants it to no existing token. The MCP OAuth approve
  stores the `super_admin` request on its code and refresh rows, and each exchange re-checks
  `password`. A new path minting a token must ask the same questions, or it reaches a different
  set of workspaces than the others.
- **Editing a token widens it only as far as the caller could mint it**
  (`tokens/update_scopes`, `tokens/update_limits` for expiration and `allowed_ips`). Narrowing is
  always allowed; a change that widens any of them (scopes outside the stored ones, a later or
  removed expiration, networks outside the stored list or none) must leave a token
  `ensure_grant_within_caller` accepts: scopes within the caller's, no superadmin unless the caller
  is unscoped and carries it, no admin the caller withholds, networks within the caller's. A later
  expiration goes through `cap_token_expiration`, and changing it re-registers the expiry warning.
  Only user tokens (`is_user_token`) take limit edits, and only they or an MCP OAuth sign-in widen
  scopes: a system token is minted narrow for wherever it is handed. Every job token is refused,
  since `$WM_TOKEN` is unscoped and narrowing breaks whatever holds the token.
- **A token's history replays from the audit log.** `users.token.create` and `users.impersonate`
  record everything the row holds (`TokenAuditGrant`), and each edit records its fields before and
  after (`update_limits` also which fields were asked for, and the requested expiration when the
  ceiling shortened it). A new token path or column must keep both, or the chain breaks. Not
  audited: the superadmin-demotion trigger, the expired-token sweep, and refused edits.
- **Every superadmin route refuses a job token**: `require_super_admin`
  (`windmill-api-auth/src/lib.rs`) errors on `authed.job_id.is_some()`. A script that needs
  `users/create`, `tokens/impersonate`, `set_login_type`, … must use a dedicated superadmin user
  token stored as a secret, never `$WM_TOKEN`. Token scopes cannot narrow superadmin routes.
- **Write access to a folder steers its readers' AI chat.** An `ai_instruction` resource's body is
  delivered to the global chat of everyone who can read the resource, with no opt-in, whenever the
  chat touches a path under its folder (`copilot/chat/folderInstructions.ts`); the chat then acts
  with that reader's permissions, outside the folder too. `ai_skill` bodies are likewise in play
  for every reader until turned off. An `ai_review_instruction` steers the AI review of every
  deployment request touching its folder, for anyone who can read it in the parent workspace
  (`deploymentRequest/aiReview.ts`); a fork's own copy is never read. The prompt tells the model to
  keep reporting secrets and unintended deletions whatever they say, but that is a request to the
  model, not enforcement: an instruction body can still talk it out of them. The resource ACL is the only
  control, so granting write on a folder grants that influence.
- **Restricted job tokens** (`job_token_scopes` on scripts and flows): a job's effective scopes
  are stored on its `job_perms` row at push and minted into its token
  (`create_token_for_owner`); an empty set is minted as `NO_API_ACCESS_SCOPE`, since
  `scopes: Some([])` reads as unscoped. Every `push` takes a `scope_ceiling`, and the job gets the
  ceiling ∩ the target's own setting: flow steps take their flow job's, agent tools their agent's,
  WAC children their parent's, retries the run they replace, and API pushes the calling job's
  (`caller_scope_ceiling`) — a scoped user or webhook token caps nothing. The row is swept once a
  job leaves the queue, so a restart of a completed flow takes the flow's current setting. Schedules,
  triggers and error/success handlers start from the target's own setting. A restricted token
  keeps only the runtime routes about its own job (`is_own_job_runtime_route`) and the reads of
  its own flow run (`flow_run_read_route_job`, checked against its lineage: the orchestrator
  evaluates a step's `results.x` with the token of the step that just finished). That lineage is
  trusted, so a restricted job token, even an admin's, can only place a job it starts in its own
  run (`unclaimable_run_lineage`); a route that
  authenticates a token itself instead of through the route layer must call
  `check_job_token_scope`. A restricted job never runs on a dedicated worker, which uses its own
  unscoped token for every job. `$var:`/`$res:` args are resolved through the API with the job's
  own token, so they need read scopes. A deploy that omits the setting keeps the deployed value
  and `null` clears it, so an unaware client cannot drop a restriction; setting one is refused
  until every worker supports it, since an older worker mints without it. The OIDC
  `job_token_scopes` claim is the presented token's scopes, not the runnable's setting, so a
  verifier sees what the job could reach and a token minted without the restriction shows as
  unrestricted. The setting belongs to
  the deployed version: an older script hash run by hash runs with that version's setting (a
  restricted caller still caps it). Write scopes on scripts, flows, schedules or triggers let a
  job escape its restriction.
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
  token only ever goes to that target. It takes no lock against what clears these rows (a removal
  from the workspace, a target change, a key rotation): their writers take the account, membership,
  key and settings rows in every order, so any lock held there could close a deadlock. A row can
  therefore land after one of them ran, however long its remote call took, and every read
  (`load_connection`) voids it instead, by identity rather than by comparing times: the connect
  records, before its remote call, the target setting's version (`remote_deploy_target_changed_at`)
  and the `created_at` of the owner's membership, and the row counts only while both are still
  the same (or the owner is a superadmin) and it decrypts. So a target set A → B → A, or a
  removal and re-add, voids it whatever the clocks say. A superadmin with no membership is bound
  through the credential making the request instead: the insert requires its `token` row to
  still exist, and `delete_user`, `leave_instance`, offboarding and SCIM removal delete an
  account's tokens, so an address deleted that way and re-created during the call does not
  inherit the connection (a JWT, having no row, cannot connect a workspace its owner is not a
  member of). A member's connect across an account re-creation is not covered: an address is not expected to pass
  to another person. A disconnect empties and stamps the row
  (inserting one if needed) rather than deleting it, `connected_at` is when a connect started,
  and the connect's upsert leaves a row stamped after that alone, so an older connect cannot undo
  a disconnect or a newer connect. Connecting by redirect: the remote's
  `/user/remote_deploy_authorize` page
  mints a token bound to the one remote workspace (`remote-deploy:<source host>`, a label
  reserved in `is_user_token` so its expiry emails nobody) only on an
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
- **`RequireSso` protection rule** (workspace-level, `check_new_member_sso` /
  `check_password_login_allowed` in `windmill-common/src/workspaces.rs`): a `password` account
  held by it (not a service account, not a bypasser by username or group, admins included) cannot
  join the workspace, and no member it holds can become a `password` account. It gates both
  transitions, never sign-in: password members already in the workspace keep logging in, which only
  the instance's `disable_password_login` stops. A new path that seats a member or writes
  `login_type = 'password'` (or a password on a `pending_oauth` account) must call the matching
  check in the transaction that writes. The two checks take no lock, so a join and a password
  switch of the same account at the same instant can both pass; the automatic joins (domain auto-add and auto-invite,
  instance groups) skip the user instead of failing, and a git sync pull into a fork fails rather
  than seat a held sync identity. The
  superadmin instance import (`POST /users/overwrite`) restores accounts as given, unchecked.
- **Login links** (`login_link` table, `POST /users/login_links` superadmin-only,
  `GET /auth/login_link/{token}` unauthenticated): single-use, ≤15 min, a session cookie and a
  302 to a same-origin `rd`. `require_login_type` on the mint refuses (409) an account whose
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
- **SSO claim names** (`sso_email_claim`, `sso_username_claim`, instance-wide, OIDC and SAML
  alike): a configured claim absent from a login falls back to the protocol default (OIDC
  `email` and the chain above, SAML NameID; `preferred_username` / the SAML username list),
  so one setting serves providers that name it differently. A configured email claim counts
  as asserted and can claim a `pending_oauth` account, so it must name an attribute users
  cannot edit at the IdP; `email_verified` is only read when the claim is `email` (the MCP
  external IdP instead refuses any token marked unverified, whatever its email claim: it never
  creates or claims an account, so the stricter rule costs nothing there). GitHub and Nextcloud
  keep their own email source. Accounts stay keyed by email: the username claim only names an
  account at creation, and an email change at the IdP signs in as a different account.
- **MCP access tokens from an external issuer** (`mcp_external_idp`, EE, `mcp/external_idp.rs`):
  taken only for a JWT whose unverified `iss` is the configured issuer, then verified against its
  discovery JWKS. With no audience configured, only a token typed as an access token (`at+jwt`
  header, or a `Bearer` `typ` claim) is taken, so a client's ID token cannot stand in for one;
  an instance-deactivated account (`password.disabled`) is refused, since this path has no
  Windmill token for deactivation to revoke. The token's client claim (`azp` by default) must name a configured client, and
  its email claim an existing, enabled member of that client's workspace: no account is created,
  and a client-credentials token (no user) is refused with 403. When `sso_groups_claim` is set,
  the instance groups with an `external_id` come from that claim of the token alone, for that
  request only (nested paths counting for their ancestors): nothing is written, unlike an SSO
  login, and an absent claim means no IdP groups, so removing a user from every group revokes at
  the next token. Roles (admin, superadmin, operator) are not derived from the token: they stay as
  stored, which SCIM and SSO login keep current. Entra's overage pointer (`_claim_names`) is refused rather than read as no
  groups. The resolved identity is cached per token for at most a minute. Runs
  record the client in `job_perms.mcp_client` (and the job-start audit entry), from the pushing identity
  rather than `job_authed`, so a run on behalf of another identity still names it; the
  session toolset reaches the API through a JWT minted for the caller, which carries the client
  in its `mcp_client` claim for the same reason, and so does a job's `WM_TOKEN`.
- **`GET /api/oauth/login/{client}`** is an unauthenticated 302 to the provider — a plain link
  from any page starts SSO.
- **`CLOUD_HOSTED`** is presence-tested (`windmill-common/src/worker.rs`): `CLOUD_HOSTED=false`
  still enables cloud mode. Of the routes above only the cloud trial offer and onboarding
  profile routes are cloud-gated; for the rest, cloud only adds quotas.
- **`CREATE_WORKSPACE_REQUIRE_SUPERADMIN`** defaults to `true` when unset; only the literal
  `"true"` enables it when set.
- **The client IP** (`windmill-common/src/client_ip.rs`, the `ClientIp` extractor) is the TCP
  peer unless that peer is in `TRUSTED_PROXIES` (comma-separated CIDRs; unset means loopback,
  plus the private ranges on cloud; empty or `none` trusts nothing). Audit entries record the
  same address (`request_origin.rs` calls it), so a login refusal and its audit row agree. A
  self-hosted instance behind a reverse proxy on a private address must list that proxy, or
  the proxy is every request's client: trusting private ranges by default would let anyone on
  the network in front name any address. Behind a trusted peer, `X-Forwarded-For`
  is read from the right up to the first untrusted hop, or, with `CLIENT_IP_HEADER` set, only that
  header (`CloudFront-Viewer-Address`, `CF-Connecting-IP`, an nginx `X-Real-IP`). `X-Real-IP` is
  never read implicitly: Caddy and most ingresses pass a client's own through. A CDN in front of
  the load balancer (CloudFront on app.windmill.dev) needs `CLIENT_IP_HEADER` or its ranges in
  `TRUSTED_PROXIES`, or its edge address is the client. On cloud, the private-range default
  also trusts job containers that can reach the server directly. Only the main API server carries the peer: a router called without connect
  info (tests, internal dispatch) sees `None`, and per-IP decisions skip or refuse it.
- **`token.allowed_ips`** is EE. A CE build drops a stored list when it loads the token, so it
  neither checks the list nor passes it on, and refuses a request that sets one. An EE build
  keeps enforcing stored lists when its license lapses; only setting a new one needs a valid
  license. The rest of this bullet describes an EE build. The column (`cidr[]`, `NULL` =
  anywhere) restricts where a token row may be
  used. `AuthCache::get_opt_job_authed` takes the request's client IP and refuses the token
  outside its networks, or when the IP is unknown, so every path that resolves a request's
  token passes that request's `ClientIp` (MCP's multi-workspace mode carries it in
  `McpToken`). The list rides on the cached `ApiAuthed`, checked per request, never folded
  into the cache entry. A restricted token cannot lift its restriction: every mint from the
  caller's identity holds the new row within the caller's networks (the caller's list when
  none is asked for). That is `create_token_internal` (`tokens/create`, app SDK and embed
  tokens, native trigger webhook tokens, which then only work from those networks),
  `tokens/impersonate` and service-account impersonation. `refresh_token`, the MCP OAuth
  approve and `login_links` refuse a restricted caller, since what they lead to is a session
  with no restriction. Not inherited, by design: a job's `WM_TOKEN` (a JWT; starting the job
  already passed the check) and JWTs the server mints for its own loopback calls.
