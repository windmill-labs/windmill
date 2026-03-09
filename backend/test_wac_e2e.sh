#!/usr/bin/env bash
# E2E test for WAC v2 workflow-as-code suspend/resume lifecycle
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8070}"
TOKEN="${WM_TOKEN:-}"
WORKSPACE="dev"
TIMEOUT=60  # seconds

# Get auth token if not set
if [ -z "$TOKEN" ]; then
    TOKEN=$(curl -s "${BASE_URL}/api/auth/login" \
        -H "Content-Type: application/json" \
        -d '{"email":"admin@windmill.dev","password":"changeme"}' | tr -d '"')
fi

echo "=== WAC v2 E2E Test ==="
echo "Base URL: $BASE_URL"
echo ""

WAC_CODE='import { task, workflow } from "windmill-client@1.999.19";

const double = task(async (x: number): Promise<number> => {
  console.log("[double] START at " + new Date().toISOString());
  await new Promise(r => setTimeout(r, 2000));
  console.log("[double] END at " + new Date().toISOString());
  return x * 2;
});

const increment = task(async (x: number): Promise<number> => {
  console.log("[increment] START at " + new Date().toISOString());
  await new Promise(r => setTimeout(r, 2000));
  console.log("[increment] END at " + new Date().toISOString());
  return x + 1;
});

export const main = workflow(async (x: number = 10) => {
  const [doubled, incremented] = await Promise.all([
    double(x),
    increment(x),
  ]);
  const final_result = await double(incremented);
  return { doubled, incremented, final_result };
});'

echo "Step 1: Submitting preview job..."
JOB_ID=$(curl -s "${BASE_URL}/api/w/${WORKSPACE}/jobs/run/preview" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d "$(jq -n --arg code "$WAC_CODE" '{
        content: $code,
        language: "bun",
        args: {"x": 10}
    }')" | tr -d '"')

echo "Job ID: $JOB_ID"

if [ -z "$JOB_ID" ] || [ "$JOB_ID" = "null" ]; then
    echo "FAIL: Could not create job"
    exit 1
fi

echo ""
echo "Step 2: Polling for completion (timeout: ${TIMEOUT}s)..."

START=$SECONDS
LAST_STATUS=""
while true; do
    ELAPSED=$((SECONDS - START))
    if [ $ELAPSED -gt $TIMEOUT ]; then
        echo "FAIL: Timed out after ${TIMEOUT}s"
        # Dump job state for debugging
        echo ""
        echo "=== Debug info ==="
        echo "Parent job queue state:"
        source /home/rfiszel/windmill__worktrees/workflows-as-code-v2/.env.local
        psql "$DATABASE_URL" -c "SELECT id, running, suspend, suspend_until, canceled_by FROM v2_job_queue WHERE id = '$JOB_ID'::uuid" 2>/dev/null
        echo "Child jobs:"
        psql "$DATABASE_URL" -c "SELECT id, running, suspend, created_at FROM v2_job_queue WHERE parent_job = '$JOB_ID'::uuid ORDER BY created_at" 2>/dev/null
        echo "Completed children:"
        psql "$DATABASE_URL" -c "SELECT id FROM completed_job WHERE parent_job = '$JOB_ID'::uuid" 2>/dev/null
        echo "Checkpoint:"
        psql "$DATABASE_URL" -c "SELECT workflow_as_code_status->'_checkpoint' FROM v2_job_status WHERE id = '$JOB_ID'::uuid" 2>/dev/null
        echo "Total child count:"
        psql "$DATABASE_URL" -c "SELECT count(*) FROM v2_job WHERE parent_job = '$JOB_ID'::uuid" 2>/dev/null
        exit 1
    fi

    # Check completed job
    RESULT=$(curl -s "${BASE_URL}/api/w/${WORKSPACE}/jobs_u/completed/get_result/${JOB_ID}" \
        -H "Authorization: Bearer $TOKEN" 2>/dev/null)
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/api/w/${WORKSPACE}/jobs_u/completed/get_result/${JOB_ID}" \
        -H "Authorization: Bearer $TOKEN" 2>/dev/null)

    if [ "$HTTP_CODE" = "200" ]; then
        echo "Job completed in ${ELAPSED}s!"
        echo ""
        echo "Step 3: Checking result..."
        echo "Result: $RESULT"

        # Validate
        DOUBLED=$(echo "$RESULT" | jq -r '.doubled // empty')
        INCREMENTED=$(echo "$RESULT" | jq -r '.incremented // empty')
        FINAL=$(echo "$RESULT" | jq -r '.final_result // empty')

        PASS=true
        if [ "$DOUBLED" != "20" ]; then
            echo "FAIL: doubled = $DOUBLED, expected 20"
            PASS=false
        fi
        if [ "$INCREMENTED" != "11" ]; then
            echo "FAIL: incremented = $INCREMENTED, expected 11"
            PASS=false
        fi
        if [ "$FINAL" != "22" ]; then
            echo "FAIL: final_result = $FINAL, expected 22"
            PASS=false
        fi

        if $PASS; then
            echo "PASS: All values correct!"
            # Check no excessive child jobs
            source /home/rfiszel/windmill__worktrees/workflows-as-code-v2/.env.local 2>/dev/null
            CHILD_COUNT=$(psql -tA "$DATABASE_URL" -c "SELECT count(*) FROM v2_job WHERE parent_job = '$JOB_ID'::uuid" 2>/dev/null)
            echo "Total child jobs created: $CHILD_COUNT (expected: 3)"
            if [ "$CHILD_COUNT" -gt "3" ]; then
                echo "WARN: More children than expected ($CHILD_COUNT > 3)"
            fi
            exit 0
        else
            exit 1
        fi
    fi

    # Show progress
    STATUS=$(curl -s "${BASE_URL}/api/w/${WORKSPACE}/jobs_u/get/${JOB_ID}" \
        -H "Authorization: Bearer $TOKEN" 2>/dev/null | jq -r '.type // empty')
    if [ "$STATUS" != "$LAST_STATUS" ]; then
        echo "  [${ELAPSED}s] Status: $STATUS"
        LAST_STATUS="$STATUS"
    fi

    # Check for runaway child creation
    source /home/rfiszel/windmill__worktrees/workflows-as-code-v2/.env.local 2>/dev/null
    CHILD_COUNT=$(psql -tA "$DATABASE_URL" -c "SELECT count(*) FROM v2_job WHERE parent_job = '$JOB_ID'::uuid" 2>/dev/null)
    if [ "$CHILD_COUNT" -gt "10" ]; then
        echo "FAIL: Runaway child creation detected! $CHILD_COUNT children (expected 3)"
        echo ""
        echo "=== Debug info ==="
        psql "$DATABASE_URL" -c "SELECT id, running, suspend, suspend_until FROM v2_job_queue WHERE id = '$JOB_ID'::uuid" 2>/dev/null
        psql "$DATABASE_URL" -c "SELECT id, running, suspend, created_at FROM v2_job_queue WHERE parent_job = '$JOB_ID'::uuid ORDER BY created_at LIMIT 20" 2>/dev/null
        psql "$DATABASE_URL" -c "SELECT workflow_as_code_status->'_checkpoint' FROM v2_job_status WHERE id = '$JOB_ID'::uuid" 2>/dev/null
        exit 1
    fi

    sleep 1
done
