# Git Sync E2E Testing Plan

## Goal

Make the git sync script **bundled with the Windmill version** (instead of stored in DB from hub).
Before that migration, we need comprehensive tests to ensure the script works correctly across all scenarios.

Two complementary strategies with **clear ownership boundaries**:

| Concern | Strategy A (Rust) | Strategy B (Docker E2E) |
|---------|------------------|------------------------|
| Filtering logic (path, type, exclude) | **Owns** | Does not test |
| Job creation & argument construction | **Owns** | Does not test |
| Settings precedence | **Owns** | Does not test |
| Draft-only skip | **Owns** | Does not test |
| Sync script produces correct git output | Does not test | **Owns** |
| Multi-repo routing (end-to-end) | Tests job fan-out | **Owns** (verifies git content) |
| Git branch modes (force_branch, individual) | Tests args passed | **Owns** (verifies branches) |
| Script update → new commit | Does not test | **Owns** |

---

## Strategy A: Rust Tests (filtering logic + job creation)

### What We're Testing

Backend logic in `windmill-git-sync/src/git_sync_ee.rs`:
- Path filtering correctness (`path_matches_filters`, `transform_regexp`)
- Type filtering (`should_skip_object_type`)
- Settings precedence (`get_effective_path_filters`, `get_effective_include_types`)
- `handle_deployment_metadata` creates correct DeploymentCallback jobs with correct args
- Multi-repo fan-out
- Draft-only skip
- Commit message formatting

### Architecture Decision: Two Test Locations

The filtering functions are **private** to `windmill-git-sync`. They cannot be tested from `windmill-api-integration-tests`. Therefore:

1. **Unit tests** → inside `windmill-git-sync/src/git_sync_ee.rs` as `#[cfg(test)] mod tests`
   - Path filtering, type filtering, settings precedence, transform_regexp

2. **Integration tests** → in `windmill-api-integration-tests/tests/git_sync.rs`
   - `handle_deployment_metadata` job creation (requires DB)
   - API endpoints (extend existing `workspaces.rs`)

### File Structure

```
backend/windmill-git-sync/src/git_sync_ee.rs        # Add #[cfg(test)] mod tests at bottom
backend/windmill-api-integration-tests/tests/
  git_sync.rs                                        # New: DB integration tests
backend/windmill-api-integration-tests/tests/fixtures/
  git_sync.sql                                       # New: workspace with git sync config
```

### Step 1: Unit tests in `windmill-git-sync`

Add a `#[cfg(test)] mod tests` block at the bottom of `git_sync_ee.rs`.
These tests are gated by `#[cfg(feature = "enterprise")]` inherently (the whole file is).

#### Test 1a: `test_transform_regexp`

Tests the glob-to-regex conversion function.

```
"f/**"  → matches "f/test/script", "f/deeply/nested/path"
"f/**"  → does NOT match "g/other", "script_at_root"
"f/*"   → matches "f/direct_child"
"f/*"   → does NOT match "f/nested/child" (single * doesn't cross /)
""      → invalid regex, returns None
"^custom.*regex$" → preserved as-is (already anchored)
```

Why: `transform_regexp` has non-trivial logic (double-replace with `%` placeholder).
A bug here silently breaks all path filtering.

#### Test 1b: `test_path_matches_filters`

Tests the 3-stage filter pipeline (include → exclude → extra_include).

```
# Basic include
include=["f/**"], path="f/test/x"           → true
include=["f/**"], path="g/test/x"           → false
include=[], path="f/test/x"                 → false (empty = nothing matches)

# Exclude
include=["f/**"], exclude=["f/internal/**"], path="f/public/x"   → true
include=["f/**"], exclude=["f/internal/**"], path="f/internal/x"  → false
include=["f/**"], exclude=None, path="f/x"                        → true
include=["f/**"], exclude=Some([]), path="f/x"                    → true (empty vec = no exclusion)

# Extra include (narrows the result)
include=["f/**"], extra=["f/special/**"], path="f/special/x"      → true
include=["f/**"], extra=["f/special/**"], path="f/other/x"        → false
include=["f/**"], extra=Some([]), path="f/other/x"                → true (empty vec = ignored)
include=["f/**"], extra=None, path="f/other/x"                    → true (None = ignored)
```

Why: The extra_include semantics (empty vec vs None vs non-empty) are subtle
and documented only in code. This is the most likely source of user-facing bugs.

#### Test 1c: `test_should_skip_object_type`

```
include=[Script, Flow], object=Script                    → false (not skipped)
include=[Script, Flow], object=App                       → true (skipped)
include=[Script], exclude_override=Some([Script])        → true (override wins)
include=[], object=Script                                → true (empty = skip all)
include=[Script], exclude_override=None                  → false
```

Why: The override interaction is easy to get wrong during refactors.

#### Test 1d: `test_effective_settings_precedence`

```
# Path filters
repo settings with non-empty include_path → repo's filters used
repo settings with empty include_path     → workspace filters used
repo settings = None                      → workspace filters used

# Type filters (same logic)
repo settings with non-empty include_type → repo's types used
repo settings with empty include_type     → workspace types used
```

Why: This precedence logic affects which objects sync. Getting it wrong
means objects silently stop syncing or sync to wrong repos.

### Step 2: Fixture file for integration tests

File: `backend/windmill-api-integration-tests/tests/fixtures/git_sync.sql`

```sql
-- Repo 1: syncs scripts+flows under f/
-- Repo 2: syncs only scripts under g/, excludes g/internal/
UPDATE workspace_settings SET git_sync = '{
  "include_path": ["f/**"],
  "include_type": ["script", "flow", "app", "folder", "resource", "variable"],
  "repositories": [
    {
      "script_path": "hub/28160/sync-script-to-git-repo-windmill",
      "git_repo_resource_path": "$res:u/test-user/git_repo_1",
      "use_individual_branch": false,
      "group_by_folder": false,
      "settings": {
        "include_path": ["f/**"],
        "include_type": ["script", "flow", "app"],
        "exclude_path": [],
        "extra_include_path": []
      }
    },
    {
      "script_path": "hub/28160/sync-script-to-git-repo-windmill",
      "git_repo_resource_path": "$res:u/test-user/git_repo_2",
      "use_individual_branch": false,
      "group_by_folder": true,
      "settings": {
        "include_path": ["g/**"],
        "include_type": ["script"],
        "exclude_path": ["g/internal/**"],
        "extra_include_path": []
      }
    }
  ]
}'::jsonb WHERE workspace_id = 'test-workspace';
```

### Step 3: Make `handle_deployment_metadata_inner` testable

The outer `handle_deployment_metadata` wraps everything in `tokio::spawn` and returns `Ok(())`.
Tests need the inner function which returns `Result<Vec<Uuid>>` synchronously.

In `windmill-git-sync/src/lib.rs`, add:
```rust
// Exposed for integration testing only
#[doc(hidden)]
#[cfg(feature = "enterprise")]
pub use git_sync_ee::handle_deployment_metadata_inner;
```

And in `git_sync_ee.rs`, change `async fn handle_deployment_metadata_inner` to `pub(crate) async fn`.

### Step 4: Add crate dependency

File: `backend/windmill-api-integration-tests/Cargo.toml`

```toml
# Add to [dependencies]
windmill-git-sync = { workspace = true }

# Update enterprise feature
enterprise = ["windmill-test-utils/enterprise", "dep:base64", "windmill-git-sync/enterprise"]
```

### Step 5: Integration tests

File: `backend/windmill-api-integration-tests/tests/git_sync.rs`

All tests gated with `#[cfg(feature = "enterprise")]`.

#### Test 5a: `test_deployment_creates_callback_job`

The core integration test — deploy an object, verify the right job is created.

```
Setup: Use git_sync fixture (2 repos: repo1=f/**, repo2=g/**)
Action: Call handle_deployment_metadata_inner() with Script at "f/test/hello"
Assert:
  - Returns exactly 1 UUID (only repo1 matches "f/**")
  - Query v2_job_queue for that UUID
  - Job args contain:
    - repo_url_resource_path = "u/test-user/git_repo_1" (stripped $res:)
    - workspace_id = "test-workspace"
    - path_type = "script"
    - commit_msg starts with "[WM]"
    - use_individual_branch = false
    - group_by_folder = false
```

Why: This is the single most important test — it validates the entire
`handle_deployment_metadata_inner` pipeline end-to-end.

#### Test 5b: `test_deployment_multi_repo_fanout`

```
Setup: Modify fixture so both repos match "f/**"
  (UPDATE workspace_settings SET git_sync = '...' with both repos having include_path=["f/**"])
Action: Deploy script at "f/shared/script"
Assert:
  - Returns 2 UUIDs
  - Each job has different repo_url_resource_path
```

Why: Multi-repo is a key enterprise feature. A bug here means
objects silently don't sync to one of the configured repos.

#### Test 5c: `test_deployment_filtered_out_creates_no_job`

Combined test for path AND type filtering (no need for separate tests).

```
Setup: Use default fixture (repo1: include_path=["f/**"], include_type=["script","flow","app"])
Actions & Asserts:
  1. Deploy Script at "g/wrong_path/x" → returns 0 UUIDs (path mismatch)
  2. Deploy Variable at "f/test/var"   → returns 0 UUIDs (type mismatch - variable not in include_type)
  3. Deploy Script at "f/test/script"  → returns 1 UUID (both match)
```

Why: Validates that filtering works in integration (not just unit tests).
Combined into one test to avoid setup duplication — each case is one function call.

#### Test 5d: `test_deployment_draft_only_skipped`

```
Setup: Insert a script with draft_only=true in the script table
Action: Call handle_deployment_metadata_inner for that script
Assert: Returns 0 UUIDs
```

Why: Draft-only skip has a DB query (checks script/flow/app table).
Unit tests can't cover this. A bug here means draft scripts get pushed to git.

#### Test 5e: `test_deployment_settings_propagation`

One test covering all settings being forwarded to job args.

```
Setup: Configure repo with:
  - use_individual_branch=true, group_by_folder=true, force_branch="staging"
  - include_type=["variable"] (no "secret")
Action: Deploy Variable at matching path
Assert job args contain:
  - use_individual_branch = true
  - group_by_folder = true
  - force_branch = "staging"
  - skip_secret = true
```

Why: Tests all arg passthrough in one shot. These are trivial `args.insert` calls
that are unlikely to break independently, so one combined test is sufficient.

#### Test 5f: `test_deployment_rename_populates_parent_path`

```
Setup: Deploy Script at "f/new_name" with parent_path=Some("f/old_name")
Assert: Job args contain path="f/new_name" AND parent_path="f/old_name"
```

Why: Rename handling is tricky — the sync script needs both paths to
delete the old file and create the new one. A bug here means orphaned files in git.

#### Test 5g: `test_deployment_commit_message_formatting`

```
Cases (all in one test):
  1. deployment_message=None         → commit_msg = "[WM] Script 'f/test/x' deployed"
  2. deployment_message="fix bug"    → commit_msg = "[WM] fix bug"
  3. deployment_message=""           → commit_msg = "[WM] Script 'f/test/x' deployed" (empty = default)
```

Why: Commit messages appear in git history. Wrong formatting is user-visible.

### Step 6: Extend existing API test

In `backend/windmill-api-integration-tests/tests/workspaces.rs`, the existing test at line 454
only checks that `edit_git_sync_config` returns 200 or 400. Extend it to also verify:

```
- POST edit_git_sync_repository with a new repo → 200, query DB to verify stored
- POST edit_git_sync_repository with existing repo → 200, verify updated
- DELETE delete_git_sync_repository → 200, verify removed
```

This extends existing code rather than creating a duplicate test file for API endpoints.

### Step 7: Run

```bash
# Unit tests (filtering logic)
cd backend
cargo test -p windmill-git-sync --features enterprise

# Integration tests (job creation)
cargo test -p windmill-api-integration-tests --features enterprise --test git_sync
```

### Test Count: 4 unit tests + 7 integration tests = 11 total

---

## Strategy B: Docker Compose E2E Tests (Python)

### What We're Testing

The **sync script itself**: given a deployed object and git repo config, does the script
produce the correct files, commits, and branches in the git repository?

Strategy A already proved that the backend creates the right jobs with the right args.
Strategy B proves the sync script (which will be bundled with the version) actually works.

### File Structure

```
integration_tests/
  docker-compose.yml                          # Modified: add Gitea service
  test/
    wmill_integration_test_utils.py           # Extended: add git sync helpers
    git_sync_test.py                          # New: E2E git sync tests
  requirements.txt                            # Add: gitpython
```

### Step 1: Add Gitea to docker-compose.yml

```yaml
gitea:
  image: gitea/gitea:1.22-rootless
  environment:
    - GITEA__database__DB_TYPE=sqlite3
    - GITEA__security__INSTALL_LOCK=true
    - GITEA__server__HTTP_PORT=3000
    - GITEA__server__ROOT_URL=http://gitea:3000
    - GITEA__service__DISABLE_REGISTRATION=false
  ports:
    - 3000:3000
  healthcheck:
    test: ["CMD-SHELL", "curl -f http://localhost:3000/api/v1/version || exit 1"]
    interval: 10s
    timeout: 5s
    retries: 10
```

The Windmill worker container needs to reach Gitea by hostname. Both are on the same
docker-compose network, so `http://gitea:3000` works from within the worker.
The test runner (host) uses `http://localhost:3000` for Gitea API and git clone.

### Step 2: Add Python dependency

Add to `integration_tests/requirements.txt`:
```
gitpython
```

### Step 3: GiteaClient helper

Add a new file or extend `wmill_integration_test_utils.py` with a `GiteaClient` class:

```python
class GiteaClient:
    def __init__(self, base_url="http://localhost:3000"):
        ...

    def setup_admin(self):
        """Create admin user 'windmill' with password 'password123!' and obtain API token."""
        # POST /api/v1/user/signup (ignore if already exists)
        # POST /api/v1/users/windmill/tokens to get token

    def create_repo(self, name) -> str:
        """Create repo, return clone URL usable from WITHIN docker network."""
        # POST /api/v1/user/repos
        # Return: "http://windmill:password123!@gitea:3000/windmill/{name}.git"

    def get_host_clone_url(self, name) -> str:
        """Return clone URL usable from the HOST (test runner)."""
        # Return: "http://windmill:password123!@localhost:3000/windmill/{name}.git"

    def delete_repo(self, name):
        # DELETE /api/v1/repos/windmill/{name}
```

Key distinction: the worker sees `gitea:3000` (docker network), the test runner sees `localhost:3000`.

### Step 4: WindmillClient extensions

Add to `WindmillClient`:

```python
def create_resource(self, path, resource_type, value, description=""):
    """POST /api/w/{workspace}/resources/create"""

def configure_git_sync(self, settings):
    """POST /api/w/{workspace}/workspaces/edit_git_sync_config"""

def wait_for_sync_jobs_complete(self, min_expected=1, timeout=90):
    """Poll completed DeploymentCallback jobs until count >= min_expected.

    Important: must check COMPLETED jobs (not just running=empty),
    because the job might not have been created yet when we start polling.
    Strategy: record job count before action, poll until count increases.
    """
```

The `wait_for_sync_jobs_complete` helper is critical. It must handle the race condition
where the `tokio::spawn` in the backend hasn't created the job yet. Implementation:

```python
def wait_for_sync_jobs_complete(self, initial_count, timeout=90):
    """Wait until completed deployment callback count exceeds initial_count."""
    start = time.time()
    while time.time() - start < timeout:
        response = self._client.get(
            f"/api/w/{self._workspace}/jobs/list",
            params={"job_kinds": "deploymentcallback", "success": "true"},
        )
        if response.status_code // 100 == 2:
            jobs = response.json()
            if len(jobs) > initial_count:
                return jobs
        time.sleep(3)
    raise Exception(f"Timed out waiting for sync jobs (have {len(jobs)}, need >{initial_count})")
```

### Step 5: Test file

File: `integration_tests/test/git_sync_test.py`

All tests share one class with `setUpClass` that creates the Gitea admin user.
Each test creates its own Gitea repo to avoid interference.

#### Test 5a: `test_script_deploy_syncs_to_git`

The core test — the first thing to get working.

```
Setup:
  1. gitea.create_repo("test-basic")
  2. wm.create_resource("u/admin/git_basic", "git_repository",
       {"url": "http://windmill:password123!@gitea:3000/windmill/test-basic.git"})
  3. wm.configure_git_sync(repositories=[{
       "script_path": "hub/28160/sync-script-to-git-repo-windmill",
       "git_repo_resource_path": "$res:u/admin/git_basic",
       "settings": {"include_path": ["f/**"], "include_type": ["script"]}
     }])

Action:
  1. initial_count = len(wm.get_completed_deployment_jobs())
  2. wm.create_script("f/test/hello", "export function main() { return 42 }", "bun")
  3. wm.wait_for_sync_jobs_complete(initial_count)

Assert:
  1. git clone http://windmill:password123!@localhost:3000/windmill/test-basic.git /tmp/clone
  2. Assert file exists matching f/test/hello (check for .ts or .script.yaml)
  3. Assert git log last commit message contains "[WM]"
```

#### Test 5b: `test_multi_repo_routing`

```
Setup:
  - gitea.create_repo("team-a-repo")
  - gitea.create_repo("team-b-repo")
  - Create 2 git resources
  - Configure 2 repos: team-a matches "f/team_a/**", team-b matches "f/team_b/**"

Action:
  - Create script at "f/team_a/script1"
  - Create script at "f/team_b/script2"
  - Wait for sync jobs

Assert:
  - Clone team-a-repo: contains team_a/script1, does NOT contain team_b/script2
  - Clone team-b-repo: contains team_b/script2, does NOT contain team_a/script1
```

Why: Multi-repo is the key enterprise differentiator. Must work end-to-end.

#### Test 5c: `test_script_update_creates_new_commit`

```
Setup: Configure git sync, create and sync initial script "f/test/versioned"
Action:
  1. Wait for first sync
  2. Update script content (POST /scripts/create with same path, different content)
  3. Wait for second sync

Assert:
  - Git repo has >= 2 commits after initial
  - Latest file content matches the updated script
```

Why: This tests the incremental sync flow (not just initial push).
A bug here means updates silently don't reach git.

#### Test 5d: `test_force_branch`

```
Setup: Configure repo with force_branch="staging"
Action: Create and sync a script

Assert:
  - Clone repo, checkout "staging" branch
  - Script file exists on "staging"
```

Why: Branch targeting is a real user workflow (staging → production promotion).
Can only be verified by inspecting actual git branches.

#### Test 5e: `test_individual_branch_mode`

```
Setup: Configure repo with use_individual_branch=true
Action: Create script at "f/test/branched"

Assert:
  - Repo has a branch other than "main" (named after the script or workspace)
  - That branch contains the script file
```

Why: Individual branch mode creates PRs per-change. Must verify actual branch creation.

#### Test 5f: `test_deploy_multiple_object_types`

One test covering script + flow + variable in a single sync repo.

```
Setup: Configure with include_type=["script", "flow", "variable"]
Action:
  - Create script at "f/test/myscript"
  - Create flow at "f/test/myflow"
  - Create variable at "f/test/myvar"
  - Wait for all sync jobs

Assert:
  - All 3 items present in git repo (check for their respective export formats)
```

Why: Verifies the sync script handles multiple object types. One combined test
instead of 3 separate tests — the sync script code path is the same, we just
need to verify it doesn't error on different types.

#### Test 5g: `test_group_by_folder`

```
Setup: Configure with group_by_folder=true
Action: Create scripts at "f/folder_a/s1" and "f/folder_b/s2"

Assert:
  - Verify the directory structure in git matches the expected grouping
    (exact structure depends on sync script behavior — may need to read script first)
```

Why: Folder grouping changes the git repo layout. Must verify actual file paths.

### Step 6: Register tests

Add to `integration_tests/test/__init__.py`:
```python
from .git_sync_test import TestGitSync
```

### Step 7: Update run.sh

Add Gitea readiness check after Windmill health check:
```bash
echo "Waiting for Gitea..."
until curl -sf http://localhost:3000/api/v1/version > /dev/null 2>&1; do
    sleep 2
done
```

### Step 8: Run

```bash
cd integration_tests
docker compose up -d
# Wait for all services
python -m pytest test/git_sync_test.py -v
```

### Test Count: 7 tests

---

## Intentionally Not Testing

These were in the original plan but removed:

| Removed Test | Why |
|-------------|-----|
| **All object types path_type mapping** (was 3p) | Trivial match arm, 20+ variants. If someone changes a string, E2E catches it. |
| **Separate tests for each arg passthrough** (was 3k, 3l, 3m) | Consolidated into one test (5e). Three `args.insert` calls don't need 3 tests. |
| **Separate API endpoint test** (was 3q) | Extend existing `workspaces.rs` test instead of duplicating. |
| **Debouncing unit test** (was 3r) | Depends on global async state (`MIN_VERSION...met()`). Hard to control in tests. |
| **Debouncing E2E** (was 5o) | Timing-dependent → flaky. Debouncing is a performance optimization, not correctness. |
| **Path/type filtering in E2E** (was 5d, 5e, 5f) | Already tested by Strategy A. E2E would just re-test the same Rust code slower. |
| **Separate flow/variable E2E** (was 5b, 5c) | Combined into one test (5f). Same sync script code path. |
| **Full workspace push** (was 5m) | Different feature (uses gitInitRepo script, not gitSync). Out of scope. |
| **Pull from git** (was 5n) | Different feature (CLI-driven, not deploy callback). Out of scope for this plan. |

---

## Summary

### Strategy A: 11 Rust tests
- 4 unit tests in `windmill-git-sync` (filtering logic — fast, no DB)
- 7 integration tests in `windmill-api-integration-tests` (job creation — needs DB)

### Strategy B: 7 Python E2E tests
- Tests the sync script produces correct git output
- Requires Docker Compose + Gitea + EE license

### Total: 18 tests with no duplication between strategies
