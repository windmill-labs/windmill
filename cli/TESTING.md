# Testing Guide for Windmill CLI Settings

This document describes the testing structure for the Windmill CLI settings functionality.

## Test Files

### Unit Tests (`settings_unit.test.ts`)

Unit tests focus on testing individual helper functions without any external dependencies:

- **UI State ↔ SyncOptions conversion functions**
- **Edge case handling**
- **Performance benchmarks**
- **Roundtrip conversion validation**

**Run unit tests:**
```bash
deno test settings_unit.test.ts --allow-all --no-check
```

### Integration Tests (`settings_integration.test.ts`)

Integration tests simulate CLI command usage without requiring a backend connection:

- **JSON parsing and validation**
- **YAML generation and file operations**
- **Diff functionality simulation**
- **CLI argument handling**
- **Error scenarios**

**Run integration tests:**
```bash
deno test settings_integration.test.ts --allow-all --no-check
```

## Features Tested

### Core Functionality
- ✅ `--from-json` JSON input parsing
- ✅ UI state to SyncOptions conversion
- ✅ SyncOptions to UI state conversion
- ✅ YAML generation and file writing
- ✅ `--diff` functionality (simulated)
- ✅ `--dry-run` mode
- ✅ Error handling for invalid JSON

### CLI Commands Tested
- ✅ `pull --from-json`
- ✅ `pull --from-json --diff`
- ✅ `pull --from-json --dry-run`
- ✅ `push --from-json`
- ✅ `push --from-json --diff`
- ✅ `push --from-json --dry-run`
- ✅ `push --diff`

### Edge Cases
- ✅ Empty UI state
- ✅ Minimal type inclusion
- ✅ Full type inclusion
- ✅ Custom include paths
- ✅ Invalid JSON input
- ✅ Missing files
- ✅ Malformed data structures

## Running All Tests

To run both unit and integration tests:
```bash
deno test settings_unit.test.ts settings_integration.test.ts --allow-all --no-check
```

To run tests through the npm test runner (for Node.js compatibility):
```bash
cd npm && npm test
```

## Test Data

The tests use realistic mock data that mirrors actual Windmill UI state:

```typescript
// Example UI State
{
  "include_path": ["f/**"],
  "include_type": ["script", "flow", "app", "folder", "variable", "resource", "resourcetype", "secret", "schedule", "trigger", "user", "group", "settings", "key"]
}

// Example SyncOptions
{
  defaultTs: 'bun',
  includes: ['f/**'],
  excludes: [],
  codebases: [],
  skipVariables: false,
  skipResources: false,
  skipResourceTypes: false,
  skipSecrets: false,
  includeSchedules: true,
  includeTriggers: true,
  includeUsers: true,
  includeGroups: true,
  includeSettings: true,
  includeKey: true
}
```

## Integration with Backend

For tests that require actual backend connectivity, they are marked with `ignore: true` and can be enabled by:

1. Setting environment variables:
   - `WMILL_TOKEN` - Your Windmill API token
   - `WMILL_BASE_URL` - Your Windmill instance URL

2. Removing the `ignore: true` flag from the test definition

## Performance Expectations

- Unit tests should complete in < 100ms total
- UI state conversions should handle 1000 operations in < 100ms
- Integration tests should complete in < 1 second total

## Adding New Tests

When adding new functionality to settings.ts:

1. Add unit tests for any helper functions in `settings_unit.test.ts`
2. Add integration tests for CLI behavior in `settings_integration.test.ts`
3. Include both positive and negative test cases
4. Test edge cases and error conditions
5. Update this documentation if new features are added