#!/usr/bin/env bash
set -euo pipefail

wm_is_true() {
  case "${1:-}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

wm_main_repo_root() {
  local repo_root=${1:-.}
  cd "$(git -C "$repo_root" rev-parse --git-common-dir 2>/dev/null)/.." && pwd
}

wm_setup_database() {
  local repo_root=$1
  local env_file=$2
  local wt_basename db_name db_conn db_url license_key

  wt_basename="$(basename "$repo_root")"
  db_name="windmill_${wt_basename//-/_}"
  db_conn="postgres://postgres:changeme@127.0.0.1:5432"

  if ! command -v psql >/dev/null 2>&1; then
    echo "WARNING: psql not found, skipping per-worktree database creation" >&2
    return
  fi

  if psql "$db_conn/postgres" -tc "SELECT 1 FROM pg_database WHERE datname = '${db_name}'" 2>/dev/null | grep -q 1; then
    echo "Database $db_name already exists"
  else
    if wm_is_true "${WM_CLONE_DB:-}"; then
      psql "$db_conn/postgres" -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'windmill' AND pid <> pg_backend_pid();" 2>/dev/null || true
      psql "$db_conn/postgres" -c "CREATE DATABASE ${db_name} TEMPLATE windmill" 2>/dev/null \
        && echo "Created database $db_name (template: windmill)" \
        || echo "WARNING: Could not create database $db_name from template windmill" >&2
    else
      psql "$db_conn/postgres" -c "CREATE DATABASE ${db_name}" 2>/dev/null \
        && echo "Created database $db_name" \
        || echo "WARNING: Could not create database $db_name (is PostgreSQL running?)" >&2
    fi
  fi

  db_url="${db_conn}/${db_name}?sslmode=disable"
  DATABASE_URL="$db_url" sqlx migrate run --source "${repo_root}/backend/migrations" \
    && echo "Migrations applied to $db_name" \
    || echo "WARNING: Could not run migrations on $db_name" >&2

  license_key="$(psql "$db_conn/windmill" -t -A -c "SELECT value FROM global_settings WHERE name = 'license_key'" 2>/dev/null || true)"
  if [[ -n "$license_key" ]]; then
    psql "$db_url" -c "INSERT INTO global_settings (name, value) VALUES ('license_key', '${license_key}'::jsonb) ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value" 2>/dev/null \
      && echo "Copied license_key to $db_name" \
      || echo "WARNING: Could not copy license_key to $db_name" >&2
  fi

  cat >> "$env_file" <<EOF
export DATABASE_URL=$db_url
WM_DB_NAME=$db_name
EOF
  echo "Added DATABASE_URL for database $db_name to .env.local"
}

wm_copy_dependencies() {
  local repo_root=$1
  local main_repo_root=$2

  if [[ -d "${main_repo_root}/frontend/node_modules" ]]; then
    cp -a "${main_repo_root}/frontend/node_modules" "${repo_root}/frontend/"
    echo "Copied frontend/node_modules (with symlinks preserved)"
  fi

  if [[ -d "${main_repo_root}/cli/node_modules" ]]; then
    cp -a "${main_repo_root}/cli/node_modules" "${repo_root}/cli/"
    echo "Copied cli/node_modules (with symlinks preserved)"
  fi

  if [[ -d "${repo_root}/cli" ]]; then
    (cd "${repo_root}/cli" && npm install && npm run gen-client) \
      && echo "CLI deps installed and client generated" \
      || echo "WARNING: CLI setup failed" >&2
  fi
}

wm_allow_direnv() {
  local repo_root=$1
  if command -v direnv >/dev/null 2>&1 && [[ -f "${repo_root}/.envrc" ]]; then
    (cd "$repo_root" && direnv allow)
    echo "direnv allowed"
  fi
}

wm_trust_claude() {
  local repo_root=$1
  local claude_json="${HOME}/.claude.json"

  if [[ ! -f "$claude_json" ]] || ! command -v python3 >/dev/null 2>&1; then
    return
  fi

  REPO_ROOT="$repo_root" CLAUDE_JSON="$claude_json" python3 - <<'PY' \
    && echo "Added $repo_root to Claude Code trusted directories" \
    || echo "Warning: Could not update Claude Code trusted directories"
import json
import os

path = os.environ["REPO_ROOT"]
claude_json = os.environ["CLAUDE_JSON"]
with open(claude_json, "r") as f:
    data = json.load(f)
projects = data.setdefault("projects", {})
proj = projects.setdefault(path, {})
proj["hasTrustDialogAccepted"] = True
proj["hasCompletedProjectOnboarding"] = True
with open(claude_json, "w") as f:
    json.dump(data, f, indent=2)
PY
}

wm_find_ee_repo() {
  local repo_root=$1
  local main_repo_root=$2
  local candidate

  for candidate in \
    "${main_repo_root}/../windmill-ee-private" \
    "${repo_root}/../windmill-ee-private" \
    "${HOME}/windmill-ee-private" \
    "${HOME}/projects/windmill-ee-private"; do
    if [[ -d "$candidate" ]]; then
      cd "$candidate" && pwd
      return 0
    fi
  done

  return 1
}

wm_setup_ee_worktree() {
  local repo_root=$1
  local main_repo_root=$2
  local ee_repo branch wt_basename ee_worktree_dir ee_rel rust_plugin

  if ! ee_repo="$(wm_find_ee_repo "$repo_root" "$main_repo_root")"; then
    return
  fi

  branch="$(git -C "$repo_root" branch --show-current 2>/dev/null || true)"
  wt_basename="$(basename "$repo_root")"
  ee_worktree_dir="${ee_repo}__worktrees/${wt_basename}"

  if [[ -n "$branch" && ! -d "$ee_worktree_dir" ]]; then
    mkdir -p "$(dirname "$ee_worktree_dir")"
    git -C "$ee_repo" fetch --quiet 2>/dev/null || true

    if git -C "$ee_repo" worktree add "$ee_worktree_dir" "$branch" 2>/dev/null; then
      echo "Created EE worktree at $ee_worktree_dir (branch: $branch)"
    elif git -C "$ee_repo" worktree add -b "$branch" "$ee_worktree_dir" main 2>/dev/null; then
      echo "Created EE worktree at $ee_worktree_dir (new branch: $branch from main)"
    else
      echo "Warning: Could not create EE worktree for branch $branch"
    fi
  elif [[ -d "$ee_worktree_dir" ]]; then
    echo "EE worktree already exists at $ee_worktree_dir"
  fi

  if [[ ! -d "$ee_worktree_dir" ]]; then
    return
  fi

  ee_rel="$(REPO_ROOT="$repo_root" EE_WORKTREE_DIR="$ee_worktree_dir" python3 - <<'PY' 2>/dev/null || echo "$ee_worktree_dir"
import os
print(os.path.relpath(os.environ["EE_WORKTREE_DIR"], os.environ["REPO_ROOT"]))
PY
)"
  mkdir -p "${repo_root}/.claude"
  rust_plugin=""
  if wm_is_true "${USE_RUST_PLUGIN:-}"; then
    rust_plugin=',
  "enabledPlugins": {
    "rust-analyzer-lsp@claude-plugins-official": true
  }'
  fi
  cat > "${repo_root}/.claude/settings.local.json" <<EOF
{
  "permissions": {
    "additionalDirectories": [
      "$ee_rel"
    ]
  }${rust_plugin}
}
EOF
  echo "Created .claude/settings.local.json with EE path: $ee_rel"

  if [[ -x "${repo_root}/backend/substitute_ee_code.sh" ]]; then
    "${repo_root}/backend/substitute_ee_code.sh" -d "$ee_worktree_dir"
  fi
}

wm_shared_post_create() {
  local repo_root=$1
  local main_repo_root

  main_repo_root="$(wm_main_repo_root "$repo_root")"
  wm_allow_direnv "$repo_root"
  wm_setup_database "$repo_root" "${repo_root}/.env.local"
  wm_copy_dependencies "$repo_root" "$main_repo_root"
  wm_trust_claude "$repo_root"
  wm_setup_ee_worktree "$repo_root" "$main_repo_root"
}

wm_kill_processes_from_env_file() {
  local env_file=$1
  local pid port

  if [[ ! -f "$env_file" ]]; then
    return
  fi

  # shellcheck disable=SC1090
  source "$env_file"
  for port in "${BACKEND_PORT:-}" "${FRONTEND_PORT:-}"; do
    [[ -z "$port" ]] && continue
    pid="$(lsof -ti "TCP:${port}" -sTCP:LISTEN 2>/dev/null || true)"
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null && echo "Killed process $pid on port $port" \
        || echo "Warning: Could not kill process $pid on port $port"
    fi
  done
}

wm_shared_pre_remove() {
  local repo_root=$1
  local env_file="${repo_root}/.env.local"
  local db_conn wt_basename main_repo_root ee_repo ee_worktree_dir

  if [[ -f "$env_file" ]]; then
    # shellcheck disable=SC1090
    source "$env_file"
  fi

  if [[ -n "${WM_DB_NAME:-}" ]]; then
    db_conn="postgres://postgres:changeme@127.0.0.1:5432"
    if command -v psql >/dev/null 2>&1; then
      psql "$db_conn/postgres" -c "DROP DATABASE IF EXISTS ${WM_DB_NAME} WITH (FORCE)" 2>/dev/null \
        && echo "Dropped database $WM_DB_NAME" \
        || echo "Warning: Could not drop database $WM_DB_NAME"
    else
      echo "psql not found, skipping database cleanup for $WM_DB_NAME"
    fi
  fi

  main_repo_root="$(wm_main_repo_root "$repo_root")"
  wt_basename="$(basename "$repo_root")"
  if ee_repo="$(wm_find_ee_repo "$repo_root" "$main_repo_root")"; then
    ee_worktree_dir="${ee_repo}__worktrees/${wt_basename}"
    if [[ -d "$ee_worktree_dir" ]]; then
      git -C "$ee_repo" worktree remove "$ee_worktree_dir" --force 2>/dev/null \
        && echo "Removed EE worktree at $ee_worktree_dir" \
        || echo "Warning: Could not remove EE worktree at $ee_worktree_dir"
    fi
  fi

  tmux kill-session -t "cursor-${wt_basename}" 2>/dev/null || true
}
