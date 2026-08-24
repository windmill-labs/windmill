#!/usr/bin/env bash
# Mint a fresh npm granular access token for the release workflow and store it as
# the NPM_TOKEN Actions secret.
#
# npm caps write-enabled granular tokens at 90 days, so this has to be re-run every
# 90 days. An expired token makes `npm publish` fail with a misleading
# `E404 ... PUT - Not found` rather than an auth error.
#
# This is a stopgap: from January 2027 npm revokes direct-publish rights from
# bypass-2FA tokens entirely, so .github/workflows/npm_on_release.yml has to move to
# trusted publishing (OIDC) before then. See https://gh.io/npm-gat-bypass2fa-deprecation
#
# Usage: scripts/rotate-npm-token.sh [--rerun]
#   --rerun   after storing the secret, re-run the failed jobs of the most recent
#             failed "Publish typescript-client & CLI to NPM on release" run

set -euo pipefail

REPO="windmill-labs/windmill"
SECRET_NAME="NPM_TOKEN"
WORKFLOW="npm_on_release.yml"
PACKAGES=(windmill-cli windmill-client)
EXPIRES_DAYS=90

RERUN=0
[[ ${1:-} == "--rerun" ]] && RERUN=1

die() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

# --- preflight -------------------------------------------------------------

command -v npm >/dev/null || die "npm not found"
command -v gh >/dev/null || die "gh not found"

# Granular-token support landed in npm 11.5.1; probe for the flag rather than
# parsing the version, since the subcommand is what actually matters.
npm token --help 2>&1 | grep -q -- '--packages-all' \
  || die "npm $(npm --version) is too old to mint granular tokens. Upgrade: npm i -g npm@latest"

gh auth status >/dev/null 2>&1 || die "gh is not authenticated. Run: gh auth login"

gh api "repos/$REPO" --jq '.permissions.admin' 2>/dev/null | grep -qx true \
  || die "your GitHub account lacks admin on $REPO, which is required to set Actions secrets"

# --- npm session -----------------------------------------------------------
# `npm login` grants a 2h session token, which is what authorizes token creation.

step "Checking npm session"
if npm_user=$(npm whoami 2>/dev/null); then
  echo "Logged in to npm as $npm_user"
else
  echo "No valid npm session (the token in ~/.npmrc is likely the expired one)."
  echo "Opening a browser login..."
  npm login
  npm_user=$(npm whoami) || die "npm login did not produce a working session"
  echo "Logged in to npm as $npm_user"
fi

# --- credentials -----------------------------------------------------------
# Passed through an npm_config_* env var rather than a --password flag so it never
# lands in argv, where any local process could read it from ps (and where `script`
# below would copy it into its log file).

step "Credentials for token creation"
read -rsp "npm password for $npm_user: " npm_config_password && echo
[[ -n $npm_config_password ]] || die "password is required"
export npm_config_password

echo "npm will challenge for 2FA next; approve it in the browser (passkey) or type the code."

# --- mint ------------------------------------------------------------------

token_name="windmill-ci-$(date +%Y-%m-%d)"
pkg_args=()
for pkg in "${PACKAGES[@]}"; do
  pkg_args+=(--packages "$pkg")
done

step "Creating granular token '$token_name'"
echo "  packages:   ${PACKAGES[*]} (read-write)"
echo "  expires:    $EXPIRES_DAYS days ($(date -d "+$EXPIRES_DAYS days" +%Y-%m-%d))"
echo "  bypass 2FA: yes (required for non-interactive CI publishes)"
echo

# npm aborts the 2FA challenge unless BOTH stdin and stdout are TTYs (see otplease
# in npm/lib/utils/auth.js), so the output cannot simply be captured with $(...).
# `script` runs npm under a real pty and tees everything to a file instead.
#
# `--json` is deliberately NOT used: npm redacts the token to "npm_***" in JSON
# output, and prints it verbatim only in the human-readable line.
npm_cmd=$(printf '%q ' npm token create \
  --name "$token_name" \
  --token-description "Publishes ${PACKAGES[*]} from $WORKFLOW" \
  --expires "$EXPIRES_DAYS" \
  "${pkg_args[@]}" \
  --packages-and-scopes-permission read-write \
  --orgs-permission no-access \
  --bypass-2fa)

# The log briefly holds a live credential, so keep it 0600 and remove it on any exit.
log_file=$(umask 077; mktemp "${TMPDIR:-/tmp}/npm-token.XXXXXX")
trap 'rm -f "$log_file"' EXIT

script -qec "$npm_cmd" "$log_file" \
  || die "npm token create failed (see the error above)"

# The token is not reliably at the start of a line: on a pty npm's preceding
# "Press ENTER to open in the browser..." prompt has no trailing newline. Grab the
# whole non-whitespace run after the marker rather than a charset, so an unexpected
# token shape fails the check below instead of being silently truncated to a
# plausible-looking value.
token=$(sed -E 's/\r/\n/g; s/\x1b\[[0-9;]*m//g' "$log_file" \
  | grep -oE 'Created token[[:space:]]+[^[:space:]]+' \
  | tail -1 \
  | sed -E 's/^Created token[[:space:]]+//')
rm -f "$log_file"

[[ $token =~ ^npm_[A-Za-z0-9]{36,48}$ ]] \
  || die "could not parse a token out of npm's output; nothing was written to GitHub"

unset npm_config_password

# --- store -----------------------------------------------------------------

step "Storing $SECRET_NAME on $REPO"
# printf, not echo: gh does not strip a trailing newline from stdin.
printf '%s' "$token" | gh secret set "$SECRET_NAME" --repo "$REPO" --app actions
unset token

updated=$(gh api "repos/$REPO/actions/secrets/$SECRET_NAME" --jq .updated_at)
echo "$SECRET_NAME updated_at: $updated"

# --- follow-up -------------------------------------------------------------

step "Done"
echo "Token expires $(date -d "+$EXPIRES_DAYS days" +%Y-%m-%d) — publishing breaks that day unless rotated again."
echo
echo "Existing npm tokens (revoke stale ones with 'npm token revoke <id>'):"
npm token list || true

if (( RERUN )); then
  step "Re-running the last failed release publish"
  read -r run_id run_tag run_date < <(gh run list --repo "$REPO" --workflow "$WORKFLOW" \
    --status failure --limit 1 --json databaseId,headBranch,createdAt \
    --jq '.[0] | "\(.databaseId) \(.headBranch) \(.createdAt)"') || true
  [[ -n ${run_id:-} ]] || die "no failed run of $WORKFLOW found"
  # Confirm rather than trusting the pick: the newest failure may be a months-old
  # release that nobody wants republished.
  echo "Most recent failed run: $run_id  tag $run_tag  $run_date"
  read -rp "Re-run its failed jobs? [y/N] " confirm
  if [[ $confirm == [yY] ]]; then
    gh run rerun "$run_id" --failed --repo "$REPO"
    echo "Watch it: gh run watch $run_id --repo $REPO"
  else
    echo "Skipped."
  fi
else
  echo
  echo "To retry the failed release publish:"
  echo "  gh run rerun \$(gh run list --repo $REPO --workflow $WORKFLOW --status failure --limit 1 --json databaseId --jq '.[0].databaseId') --failed --repo $REPO"
fi
