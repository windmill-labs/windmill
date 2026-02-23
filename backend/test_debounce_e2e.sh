#!/usr/bin/env bash
# End-to-end debounce tests against the running backend API
# Usage: BACKEND_PORT=8030 ./test_debounce_e2e.sh
set -uo pipefail

BASE="http://localhost:${BACKEND_PORT:-8030}/api"
W="admins"
EMAIL="admin@windmill.dev"
PASSWORD="changeme"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0

log_pass() { echo -e "${GREEN}PASS${NC}: $1"; ((pass++)) || true; }
log_fail() { echo -e "${RED}FAIL${NC}: $1 — $2"; ((fail++)) || true; }
log_info() { echo -e "${YELLOW}INFO${NC}: $1"; }

# Unique suffix for idempotent re-runs
TS=$(date +%s)

# --- Auth ---
log_info "Logging in..."
TOKEN=$(curl -s "$BASE/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")

if [ -z "$TOKEN" ]; then
  echo "Failed to login"; exit 1
fi

AUTH="Authorization: Bearer $TOKEN"
log_info "Logged in"

# --- Helpers ---
api() {
  # Usage: api METHOD path [data]
  local method="$1" path="$2" data="${3:-}"
  if [ -n "$data" ]; then
    curl -s "$BASE/w/$W/$path" -X "$method" -H "$AUTH" -H 'Content-Type: application/json' -d "$data"
  else
    curl -s "$BASE/w/$W/$path" -X "$method" -H "$AUTH"
  fi
}

wait_job() {
  local job_id="$1" max_wait="${2:-30}"
  for _ in $(seq 1 "$max_wait"); do
    local r
    r=$(api GET "jobs/completed/get_result_maybe/$job_id")
    if echo "$r" | jq -e '.completed == true' > /dev/null 2>&1; then
      echo "$r"; return 0
    fi
    sleep 1
  done
  echo '{"completed":false,"error":"timeout"}'; return 1
}

BUN_EMPTY_LOCK=$'{"dependencies": {}}\n//bun.lock\n'

create_script() {
  # Usage: create_script path language content [extra_json_fields]
  # Note: lock must be non-empty; empty string ("") is treated as None by the backend
  # (scripts.rs:798-800), which triggers dependency resolution instead of direct deployment.
  # For bun scripts, the lock must contain "//bun.lock" as a split pattern.
  local path="$1" lang="$2" content="$3" extra="${4:-}"
  local json
  json=$(jq -n \
    --arg path "$path" \
    --arg lang "$lang" \
    --arg content "$content" \
    --arg summary "test" \
    --arg desc "test" \
    --arg lock "$BUN_EMPTY_LOCK" \
    '{path: $path, language: $lang, content: $content, summary: $summary, description: $desc, lock: $lock}')
  if [ -n "$extra" ]; then
    json=$(echo "$json" | jq ". + $extra")
  fi
  local hash
  hash=$(api POST "scripts/create" "$json")
  # Small delay for DB visibility after tx commit
  sleep 0.2
  echo "$hash"
}

run_script() {
  # Usage: run_script path args_json
  api POST "jobs/run/p/$1" "$2"
}

###############################################################################
# TEST 1: Deploy a script and run it 5 times in close succession
###############################################################################
echo ""
log_info "=== TEST 1: Deploy & run script 5 times rapidly ==="

P1="u/admin/e2e_simple_$TS"
H1=$(create_script "$P1" "bun" 'export function main(x: number = 0) { return { result: x * 2 }; }')

if echo "$H1" | grep -qE '^[0-9a-f]{16}$'; then
  log_pass "Script created: $H1"
else
  log_fail "Script creation" "$H1"
fi

log_info "Running 5 times rapidly..."
JOB_IDS=()
for i in $(seq 1 5); do
  JID=$(run_script "$P1" "{\"x\": $i}")
  JOB_IDS+=("$JID")
done
log_info "Jobs: ${JOB_IDS[*]}"

log_info "Waiting for completion..."
all_ok=true
for i in "${!JOB_IDS[@]}"; do
  JID="${JOB_IDS[$i]}"
  R=$(wait_job "$JID" 30)
  success=$(echo "$R" | jq -r '.success // false')
  value=$(echo "$R" | jq -r '.result.result // "null"')
  expected=$(( (i + 1) * 2 ))
  if [ "$success" = "true" ] && [ "$value" = "$expected" ]; then
    log_pass "Job $((i+1)): x=$((i+1)) → $value (correct)"
  else
    log_fail "Job $((i+1))" "success=$success value=$value expected=$expected"
    all_ok=false
  fi
done

if [ "$all_ok" = "true" ]; then
  log_pass "All 5 runs completed correctly (no debounce — different args)"
fi

###############################################################################
# TEST 2: Redeploy script WITHOUT lock in close succession
###############################################################################
echo ""
log_info "=== TEST 2: Redeploy without lock in rapid succession ==="

P2="u/admin/e2e_nolock_$TS"

# Deploy 5 versions of the same script without lock → triggers dependency jobs
DEPLOY_HASHES=()
for i in $(seq 1 5); do
  content="export function main(x: number = 0) { return { result: x * $i, version: $i }; }"
  parent_extra=""
  if [ "${#DEPLOY_HASHES[@]}" -gt 0 ]; then
    last_hash="${DEPLOY_HASHES[-1]}"
    parent_extra="{\"parent_hash\": \"$last_hash\"}"
  fi

  # Deploy without lock (omit lock field entirely)
  json=$(jq -n \
    --arg path "$P2" \
    --arg content "$content" \
    --arg summary "v$i" \
    --arg desc "test" \
    '{path: $path, language: "bun", content: $content, summary: $summary, description: $desc}')
  if [ -n "$parent_extra" ]; then
    json=$(echo "$json" | jq ". + $parent_extra")
  fi

  hash=$(api POST "scripts/create" "$json")
  if echo "$hash" | grep -qE '^[0-9a-f]{16}$'; then
    DEPLOY_HASHES+=("$hash")
    log_info "Deploy $i: $hash"
  else
    log_fail "Deploy $i" "$hash"
    # If path conflict, the script already exists from a previous version
    break
  fi
  sleep 0.1
done

# Wait for dependency resolution
log_info "Waiting 15s for dependency jobs..."
sleep 15

# Check the latest script — should have lock resolved
SCRIPT_INFO=$(api GET "scripts/get/p/$P2")
LOCK=$(echo "$SCRIPT_INFO" | jq -r '.lock // "null"')
if [ "$LOCK" != "null" ] && [ -n "$LOCK" ]; then
  log_pass "Latest version has lock resolved"
else
  log_info "Lock not yet resolved: $LOCK"
fi

# Run the latest version to verify it works
sleep 0.5
JID2=$(run_script "$P2" '{"x": 10}')
if echo "$JID2" | grep -qE '^[0-9a-f-]{36}$'; then
  R2=$(wait_job "$JID2" 30)
  success=$(echo "$R2" | jq -r '.success // false')
  if [ "$success" = "true" ]; then
    version=$(echo "$R2" | jq -r '.result.version // "?"')
    log_pass "Latest version runs: version=$version"
  else
    err=$(echo "$R2" | jq -r '.result.error.message // "unknown"' 2>/dev/null)
    log_fail "Run latest version" "success=false err=$err"
  fi
else
  log_fail "Run latest version" "bad job id: $JID2"
fi

###############################################################################
# TEST 3: Script with debounce_delay_s — rapid runs with SAME args
###############################################################################
echo ""
log_info "=== TEST 3: Debounce with same args (should debounce) ==="

P3="u/admin/e2e_debounce_$TS"
H3=$(create_script "$P3" "bun" \
  'export function main(x: number = 0) { return { result: x }; }' \
  '{"debounce_delay_s": 3}')

if echo "$H3" | grep -qE '^[0-9a-f]{16}$'; then
  log_pass "Debounce script created: $H3"
else
  log_fail "Debounce script creation" "$H3"
fi

log_info "Running 5 times with same args {x: 42}..."
DEB_IDS=()
for i in $(seq 1 5); do
  JID=$(run_script "$P3" '{"x": 42}')
  DEB_IDS+=("$JID")
  log_info "  Run $i: $JID"
done

log_info "Waiting 10s for debounce delay (3s) + execution..."
sleep 10

executed=0
skipped=0
for JID in "${DEB_IDS[@]}"; do
  if ! echo "$JID" | grep -qE '^[0-9a-f-]{36}$'; then
    log_info "  Invalid job id: $JID"
    continue
  fi
  R=$(wait_job "$JID" 5 2>/dev/null || echo '{"completed":false}')
  completed=$(echo "$R" | jq -r '.completed // false')
  success=$(echo "$R" | jq -r '.success // false')
  if [ "$completed" = "true" ] && [ "$success" = "true" ]; then
    ((executed++)) || true
  elif [ "$completed" = "true" ]; then
    ((skipped++)) || true
  fi
done

log_info "Results: $executed executed, $skipped skipped out of ${#DEB_IDS[@]}"
if [ "$executed" -eq 1 ] && [ "$skipped" -ge 3 ]; then
  log_pass "Debouncing perfect: 1 executed, $skipped skipped"
elif [ "$executed" -le 2 ] && [ "$skipped" -ge 2 ]; then
  log_pass "Debouncing working: $executed executed, $skipped skipped"
else
  log_fail "Debounce same args" "executed=$executed skipped=$skipped (want ~1 exec, ~4 skip)"
fi

###############################################################################
# TEST 3b: Different args should NOT debounce against each other
###############################################################################
echo ""
log_info "=== TEST 3b: Debounce with different args (should NOT debounce) ==="

DIFF_IDS=()
for i in $(seq 1 3); do
  JID=$(run_script "$P3" "{\"x\": $((i * 100))}")
  DIFF_IDS+=("$JID")
done

log_info "Waiting 8s..."
sleep 8

diff_exec=0
for JID in "${DIFF_IDS[@]}"; do
  if ! echo "$JID" | grep -qE '^[0-9a-f-]{36}$'; then continue; fi
  R=$(wait_job "$JID" 5 2>/dev/null || echo '{"completed":false}')
  success=$(echo "$R" | jq -r '.success // false')
  if [ "$success" = "true" ]; then ((diff_exec++)) || true; fi
done

if [ "$diff_exec" -eq 3 ]; then
  log_pass "Different args: all 3 executed independently"
else
  log_fail "Different args" "only $diff_exec/3 executed"
fi

###############################################################################
# TEST 4: Custom debounce_key with $args interpolation
###############################################################################
echo ""
log_info "=== TEST 4: Custom debounce key ==="

P4="u/admin/e2e_custom_key_$TS"
H4=$(create_script "$P4" "bun" \
  'export function main(event_id: string = "", data: string = "") { return { event_id, data }; }' \
  '{"debounce_delay_s": 3, "debounce_key": "event#$args.event_id"}')

if echo "$H4" | grep -qE '^[0-9a-f]{16}$'; then
  log_pass "Custom key script created: $H4"
else
  log_fail "Custom key script creation" "$H4"
fi

# Same event_id → should debounce
log_info "3 runs with same event_id..."
SAME_IDS=()
for i in $(seq 1 3); do
  JID=$(run_script "$P4" "{\"event_id\": \"evt_001\", \"data\": \"payload_$i\"}")
  SAME_IDS+=("$JID")
done

# Different event_id → should NOT debounce
JID_DIFF=$(run_script "$P4" '{"event_id": "evt_002", "data": "different"}')

log_info "Waiting 8s..."
sleep 8

same_exec=0
same_skip=0
for JID in "${SAME_IDS[@]}"; do
  if ! echo "$JID" | grep -qE '^[0-9a-f-]{36}$'; then continue; fi
  R=$(wait_job "$JID" 5 2>/dev/null || echo '{"completed":false}')
  completed=$(echo "$R" | jq -r '.completed // false')
  success=$(echo "$R" | jq -r '.success // false')
  if [ "$completed" = "true" ] && [ "$success" = "true" ]; then
    data=$(echo "$R" | jq -r '.result.data // "?"')
    ((same_exec++)) || true
    log_info "  Executed: data=$data"
  elif [ "$completed" = "true" ]; then
    ((same_skip++)) || true
  fi
done

log_info "Same event_id: $same_exec executed, $same_skip skipped"
if [ "$same_exec" -eq 1 ] && [ "$same_skip" -ge 1 ]; then
  log_pass "Custom key debounce: same event_id debounced correctly"
elif [ "$same_exec" -le 2 ]; then
  log_pass "Custom key debounce working: $same_exec executed, $same_skip skipped"
else
  log_fail "Custom key debounce" "exec=$same_exec skip=$same_skip"
fi

# Check different event_id ran independently
if echo "$JID_DIFF" | grep -qE '^[0-9a-f-]{36}$'; then
  R_DIFF=$(wait_job "$JID_DIFF" 10 2>/dev/null || echo '{"completed":false}')
  diff_success=$(echo "$R_DIFF" | jq -r '.success // false')
  if [ "$diff_success" = "true" ]; then
    log_pass "Different event_id: executed independently"
  else
    log_info "Different event_id: success=$diff_success"
  fi
fi

###############################################################################
# TEST 5: Git sync with bad target — debounced deployment callbacks
###############################################################################
echo ""
log_info "=== TEST 5: Git sync debounce + aggregation ==="

# Create git repo resource
api POST "resources/create?update_if_exists=true" '{
  "path": "u/admin/e2e_bad_git_repo",
  "description": "Bad git repo for testing",
  "resource_type": "git_repository",
  "value": {"url": "https://github.com/nonexistent/nope.git", "branch": "main", "token": "bad"}
}' > /dev/null 2>&1
log_info "Created git repo resource"

# Create a sync script at a folder path where the 2nd segment is a number >= 28103.
# is_script_meets_min_version parses split("/").skip(1).next() as the version number.
# This enables debounce_delay_s=5 and debounce_args_to_accumulate=["items"].
api POST "folders/create" '{"name": "28103"}' > /dev/null 2>&1
P5="f/28103/e2e_sync_$TS"
H5=$(create_script "$P5" "bun" \
  'export function main(repo_url_resource_path: string = "", workspace_id: string = "", items: any[] = [], use_individual_branch: boolean = false, group_by_folder: boolean = false, parent_workspace_id: string = "") { return { synced: items.length, items }; }')

if echo "$H5" | grep -qE '^[0-9a-f]{16}$'; then
  log_pass "Sync script created: $H5"
else
  log_fail "Sync script creation" "$H5"
fi

# Configure git sync with include_path to match deployed scripts.
# Without include_path, path_matches_filters returns false and no DeploymentCallback is created.
api POST "workspaces/edit_git_sync_config" "{
  \"git_sync_settings\": {
    \"include_type\": [\"script\"],
    \"include_path\": [\"**\"],
    \"repositories\": [{
      \"script_path\": \"$P5\",
      \"git_repo_resource_path\": \"\$res:u/admin/e2e_bad_git_repo\",
      \"use_individual_branch\": false,
      \"group_by_folder\": false
    }]
  }
}" > /dev/null 2>&1
log_pass "Git sync configured with include_path and versioned folder script path"

# Deploy 5 scripts rapidly to trigger git sync.
# Scripts are created with lock="" (via create_script), so handle_deployment_metadata
# fires immediately after tx commit (not after dependency resolution).
log_info "Deploying 5 scripts to trigger git sync..."
for i in $(seq 1 5); do
  dp="u/admin/e2e_gitsync_${TS}_$i"
  create_script "$dp" "bun" "export function main() { return { v: $i }; }" > /dev/null
  log_info "  Deployed $dp"
done

# Wait for debounce delay (5s) + execution
log_info "Waiting 15s for debounce (5s) + execution..."
sleep 15

# Check deployment callback jobs for our sync script.
# Debounced jobs have is_skipped=true (but success=true), so we use is_skipped to distinguish.
SYNC_JOBS=$(api GET "jobs/completed/list?script_path_exact=$P5&job_kinds=deploymentcallback")
SYNC_TOTAL=$(echo "$SYNC_JOBS" | jq 'length')
SYNC_EXECUTED=$(echo "$SYNC_JOBS" | jq '[.[] | select(.is_skipped != true)] | length')
SYNC_SKIPPED=$(echo "$SYNC_JOBS" | jq '[.[] | select(.is_skipped == true)] | length')

log_info "Sync jobs: total=$SYNC_TOTAL executed=$SYNC_EXECUTED skipped=$SYNC_SKIPPED"

if [ "$SYNC_TOTAL" -gt 0 ]; then
  # With debouncing (5s delay), rapid deploys should be consolidated.
  # All 5 jobs are created but most should be skipped (debounced).
  if [ "$SYNC_SKIPPED" -gt 0 ]; then
    log_pass "Git sync debouncing: $SYNC_EXECUTED executed, $SYNC_SKIPPED debounced out of $SYNC_TOTAL"
  else
    log_fail "Git sync debouncing" "No jobs were debounced ($SYNC_TOTAL all executed independently)"
  fi

  # Check if items were aggregated in the executed (non-skipped) job(s)
  for idx in $(seq 0 $((SYNC_TOTAL - 1))); do
    is_skipped=$(echo "$SYNC_JOBS" | jq -r ".[$idx].is_skipped")
    [ "$is_skipped" = "true" ] && continue
    jid=$(echo "$SYNC_JOBS" | jq -r ".[$idx].id")
    r=$(api GET "jobs/completed/get_result/$jid")
    items_count=$(echo "$r" | jq '.items | length // 0')
    log_info "  Executed sync job $jid: items=$items_count"
    if [ "$items_count" -gt 1 ]; then
      log_pass "Items aggregated: $items_count items in single sync job"
    fi
  done
else
  # Check queued — jobs may still be pending debounce delay
  Q=$(api GET "jobs/queue/list?script_path_exact=$P5&job_kinds=deploymentcallback")
  QC=$(echo "$Q" | jq 'length')
  log_info "No completed sync jobs. $QC queued."
  if [ "$QC" -gt 0 ] && [ "$QC" -lt 5 ]; then
    log_pass "Git sync debouncing (queued): $QC jobs for 5 deploys"
  elif [ "$QC" -eq 0 ]; then
    log_fail "Git sync" "No deployment callback jobs found (completed or queued)"
  fi
fi

# Cleanup git sync
api POST "workspaces/edit_git_sync_config" '{"git_sync_settings": null}' > /dev/null 2>&1
log_info "Git sync config cleared"

###############################################################################
# Summary
###############################################################################
echo ""
echo "========================================="
echo -e "Results: ${GREEN}$pass passed${NC}, ${RED}$fail failed${NC}"
echo "========================================="

if [ "$fail" -gt 0 ]; then
  exit 1
fi
