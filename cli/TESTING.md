# Testing Guide for Windmill CLI

## Running Tests

```bash
# Run all tests
deno test -A --no-check test/

# Run specific test files
deno test -A --no-check test/gitsync_settings_features.test.ts
deno test -A --no-check test/init_no_git_sync.test.ts
deno test -A --no-check test/multi_instance_workspace.test.ts
deno test -A --no-check test/override_settings_behavior.test.ts
deno test -A --no-check test/sync_config_resolution.test.ts
deno test -A --no-check test/workspace_conflicts.test.ts

# Run with specific test patterns
deno test -A --no-check test/ --filter "workspace"
deno test -A --no-check test/ --filter "sync"
```

## Test Files

- **`gitsync_settings_features.test.ts`** - Git sync settings functionality
- **`init_no_git_sync.test.ts`** - Init without git sync
- **`multi_instance_workspace.test.ts`** - Multi-instance workspace handling
- **`override_settings_behavior.test.ts`** - Settings override behavior
- **`sync_config_resolution.test.ts`** - Sync configuration resolution
- **`workspace_conflicts.test.ts`** - Workspace conflict detection

## Docker Requirements

```bash
# Ensure Docker is running
docker --version
docker-compose --version

# Ensure EE license key is available
echo $EE_LICENSE_KEY
```

## Debugging Failed Tests

```bash
# Run with verbose output
deno test -A --no-check test/ --reporter=verbose

# Check container status
docker ps

# View backend logs
docker logs test-test_windmill_server-1

# Manual container management
cd test
docker compose -f docker-compose.test.yml up -d
docker compose -f docker-compose.test.yml down
docker compose -f docker-compose.test.yml down -v
```