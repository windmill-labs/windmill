# CLI Refactoring Plan: Simplifying Git-Sync Settings Management

## Overview

This plan outlines the refactoring of the Windmill CLI to simplify how git-sync settings are managed, removing complexity from sync commands and creating a cleaner separation of concerns.

## Key Principles

1. **Sync commands focus only on file synchronization** - no git-sync settings management
2. **Settings commands handle all git-sync configuration** - pull/push settings between local and backend
3. **Repository selection happens at runtime** - not stored in wmill.yaml
4. **Hierarchical configuration** - top-level defaults with workspace and repository-specific overrides

## Implementation Strategy

### Start Fresh Approach

Given the scope of changes, we'll start from a clean main branch and selectively bring over useful features:

```bash
# Create backup of current work
cp -r cli cli-bak

# Reset cli directory to main branch
cd cli
git checkout main -- .

# Implement changes incrementally
```

### Features to Preserve from Current Branch

From `cli-bak`, we'll selectively copy over:
- Settings diff logic (`--diff` flag implementation)
- JSON output functionality (`jsonOutput` option)
- Network error handling improvements in `context.ts`
- The token fix in `requireLogin()` (`newToken` instead of `token`)
- The new skip options (`skipScripts`, `skipFlows`, `skipApps`, `skipFolders`)
- Testing infrastructure (can be added in a separate PR)

## Proposed Changes

### 1. Configuration Structure (conf.ts)

#### Remove:
- `includeWmillYaml` from `SyncOptions`
- `settingsFromJson` from `SyncOptions`
- `defaultWorkspace` from `SyncOptions`
- `repositories` from workspace configuration
- `WorkspaceProfile` interface (replace with simpler override structure)
- `RepositorySyncOptions` interface (entire interface)
- Helper functions that deal with old structure:
  - `getWorkspaceProfile()`
  - `getWorkspaceRepositorySettings()`
  - `listWorkspaces()`
  - `listWorkspaceRepositories()`
- Complex `defaultTs` checking logic in `readConfigFile()`

#### Add/Modify:
- Add `skipScripts`, `skipFlows`, `skipApps`, `skipFolders` to `SyncOptions`
- Add `jsonOutput` to `SyncOptions`
- Add `overrides` field in `SyncOptions` as a map of settings keyed by `<workspace>:<repo>`
- New helper function `getEffectiveSettings(workspace: string, repo: string)` that merges:
  1. Top-level defaults
  2. Workspace-level overrides (`workspace:*`)
  3. Repository-specific overrides (`workspace:repo`)
- Keep `DEFAULT_SYNC_OPTIONS` constant

### 2. Sync Commands (sync.ts)

#### Remove:
- `--include-wmill-yaml` flag from pull/push commands
- `--settings-from-json` flag from pull/push commands
- Any logic that deals with fetching or applying backend settings
- Any logic that modifies wmill.yaml based on backend settings

#### Behavior:
- `wmill sync pull` - pulls files from workspace based on effective settings
- `wmill sync push` - pushes files to workspace based on effective settings
- Both commands use merged settings from `getEffectiveSettings()`

### 3. Settings Commands (settings.ts)

#### New/Enhanced Functionality:

**`wmill settings pull`**
- Fetches git-sync settings from backend
- Repository selection:
  - If only 1 repository configured → use it automatically
  - If multiple repositories → prompt user interactively (or use `--repository` flag)
- Writes settings to local wmill.yaml:
  - Default: writes to `overrides['workspace:repo']`
  - With `--workspace-level` flag: writes to `overrides['workspace:*']`
  - With `--default` flag: writes to top-level defaults
- Does NOT include repository information in settings themselves

**`wmill settings push`**
- Reads local wmill.yaml effective settings for current workspace/repo
- Repository selection (same logic as pull)
- Pushes settings to the selected repository in backend
- Updates only the settings for that specific repository

**Common flags for settings commands:**
- `--repository <path>` - specify repository path (e.g., `u/user/repo`)
- `--workspace <name>` - override current workspace
- `--workspace-level` - for pull, write settings to workspace:* override
- `--default` - for pull, write settings to top-level defaults

### 4. Init Command (init.ts)

#### Enhanced Behavior:
- When initializing with an existing workspace (`wmill init`):
  - Check if workspace has git-sync configured
  - If yes, automatically run `wmill settings pull` logic:
    - Single repo → pull settings automatically
    - Multiple repos → prompt for selection
    - Write settings to appropriate override key
- Create initial project structure
- The `--repository` flag can be passed to `wmill init` for non-interactive selection

### 5. Error Handling (context.ts)

#### Keep all improvements:
- Network error detection with clear messages
- Token fix (`newToken` instead of `token`)
- Response body consumption for error cases

### 6. wmill.yaml Format Examples

#### Example 1: Simple (single workspace/repo) - Current format still valid!
```yaml
# Top-level defaults - this format remains valid
includes: ["f/**"]
excludes: ["**/.git/**"]
defaultTs: bun
skipVariables: false
skipResources: false
skipSecrets: true
skipScripts: false
skipFlows: false
skipApps: false
includeSchedules: true
```

#### Example 2: Multi-workspace/repo with overrides
```yaml
# Top-level defaults apply to all workspaces/repos
includes: ["f/**"]
excludes: ["**/.git/**"]
defaultTs: bun
skipVariables: false
skipResources: false
skipSecrets: true

# Override structure: <workspace>:<repo> as keys
overrides:
  # Dev workspace overrides
  "dev:u/alice/frontend":
    includes: ["f/**", "u/alice/**"]
    skipSecrets: false
    defaultTs: deno
    
  "dev:u/alice/backend":
    includes: ["backend/**"]
    skipFlows: true
    skipApps: true
    
  # Production workspace - more restrictive
  "prod:u/team/main":
    skipVariables: true
    skipResources: true
    includeSchedules: true
    includeTriggers: true
    
  # Different workspace URL
  "self-hosted:u/internal/scripts":
    baseUrl: https://windmill.company.com
    includes: ["internal/**", "shared/**"]
    skipSecrets: false
```

#### Example 3: Workspace-level and repo-level overrides
```yaml
# Top-level defaults
includes: ["f/**"]
defaultTs: bun

overrides:
  # Workspace-level override (applies to all repos in staging)
  "staging:*":
    includeSchedules: false
    skipSecrets: false
    
  # Specific repo override (takes precedence over workspace-level)
  "staging:u/alice/critical":
    includeSchedules: true  # Override the staging:* setting
    skipVariables: true
    
  # Another workspace with defaults
  "prod:*":
    skipSecrets: true
    skipVariables: true
    
  "prod:u/team/public-scripts":
    skipSecrets: true  # Redundant but explicit
    includes: ["public/**"]
```

#### Example 4: Multiple workspaces with shared repos
```yaml
includes: ["f/**"]
defaultTs: bun

overrides:
  # Alice's repos across different workspaces
  "dev:u/alice/common":
    includes: ["f/**", "shared/**"]
    skipSecrets: false
    
  "staging:u/alice/common":
    includes: ["f/**", "shared/**"]
    skipSecrets: true  # More restrictive in staging
    
  # Team repos with workspace-specific settings
  "dev:u/team/backend":
    includes: ["backend/**", "tests/**"]
    
  "prod:u/team/backend":
    includes: ["backend/**"]  # No tests in prod
    skipVariables: true
```

## Settings Resolution Logic

The `getEffectiveSettings(workspace: string, repo: string)` function will:

1. Start with `DEFAULT_SYNC_OPTIONS`
2. Merge top-level settings from wmill.yaml
3. Apply workspace-level overrides from `overrides['workspace:*']` if exists
4. Apply repo-specific overrides from `overrides['workspace:repo']` if exists
5. Return the merged settings

Priority order (highest to lowest):
- `overrides['workspace:repo']` (most specific)
- `overrides['workspace:*']` (workspace-wide)
- Top-level settings in wmill.yaml
- `DEFAULT_SYNC_OPTIONS` (built-in defaults)

## Implementation Steps

### Phase 0: Setup Fresh Start
1. Create backup: `cp -r cli cli-bak`
2. Reset to main: `git checkout main -- cli/`
3. Create feature branch for clean implementation

### Phase 1: Core Configuration Changes
1. Add new fields to `SyncOptions`:
   - `skipScripts`, `skipFlows`, `skipApps`, `skipFolders`
   - `jsonOutput`
   - `overrides` map
2. Implement `getEffectiveSettings()` function
3. Update `DEFAULT_SYNC_OPTIONS` with new fields

### Phase 2: Settings Commands Enhancement
1. Copy over repository selection logic from `cli-bak/settings.ts`
2. Implement interactive repository selection
3. Update `pullSettings()` to:
   - Use selected repository
   - Write to appropriate override key
4. Update `pushSettings()` to:
   - Use effective settings
   - Target selected repository
5. Copy over diff logic from `cli-bak`

### Phase 3: Sync Commands Simplification
1. Remove git-sync related flags
2. Update to use `getEffectiveSettings()`
3. Copy over JSON output logic from `cli-bak`
4. Ensure clean separation from settings management

### Phase 4: Init Command Update
1. Add git-sync detection logic
2. Integrate settings pull flow
3. Support `--repository` flag

### Phase 5: Error Handling Improvements
1. Copy over network error handling from `cli-bak/context.ts`
2. Apply token fix in `requireLogin()`
3. Add response body consumption

### Phase 6: Testing and Documentation
1. Update existing tests
2. Add new tests for override logic
3. Update documentation
4. Create migration guide

## Benefits

1. **Clear hierarchy**: defaults → workspace:* → workspace:repo
2. **Flexible**: Can set workspace-wide policies with specific repo exceptions
3. **Readable**: The key format clearly shows the scope
4. **Simple lookup**: Easy to find settings for any workspace/repo combination
5. **Clean separation**: Sync commands don't deal with settings management
6. **Better UX**: Automatic repository selection when possible
7. **Backward compatible**: Existing wmill.yaml files continue to work

## Migration Path

**No migration needed!** The current format remains valid:
- Existing wmill.yaml files with just top-level settings continue to work as-is
- The `overrides` section is optional and only added when:
  - User explicitly pulls settings for a specific workspace/repo
  - User wants different settings for different workspaces/repos
- Old format = new format with no overrides

For users who want to adopt overrides:
1. Continue using existing wmill.yaml
2. Use `wmill settings pull` with appropriate flags to add overrides
3. Manually edit wmill.yaml to add overrides section if preferred

## Future Enhancements

1. Support for patterns in override keys (e.g., `*:u/alice/*`)
2. `wmill settings list` command to show available repositories
3. `wmill settings diff` to compare local vs backend settings
4. Validation of settings before push
5. `wmill settings show` to display effective settings for current context

## Implementation Progress

### Phase 0: Setup Fresh Start
- [x] Create backup: `cp -r cli cli-bak`
- [x] Reset to main: `git checkout main -- cli/`
- [x] Stay on current branch (no new feature branch needed)

### Phase 1: Core Configuration Changes
- [x] Add `skipScripts`, `skipFlows`, `skipApps`, `skipFolders` to `SyncOptions`
- [x] Add `jsonOutput` to `SyncOptions`
- [x] Add `overrides` map to `SyncOptions`
- [x] Implement `getEffectiveSettings()` function
- [x] Update `DEFAULT_SYNC_OPTIONS` with new fields
- [ ] Remove old interfaces (`WorkspaceProfile`, `RepositorySyncOptions`) - N/A (not present in current version)
- [ ] Remove old helper functions - N/A (not present in current version)
- [ ] Simplify `readConfigFile()` logic - N/A (already simple)

### Phase 2: Settings Commands Enhancement
- [x] Copy repository selection logic from `cli-bak/settings.ts` - Implemented fresh
- [x] Implement interactive repository selection
- [x] Update `pullSettings()` for new structure - Created new gitsync-settings pull
- [x] Update `pushSettings()` for new structure - Created new gitsync-settings push
- [x] Add `--repository` flag support
- [x] Add `--workspace-level` flag support
- [x] Add `--default` flag support
- [x] Copy over diff logic from `cli-bak` - Implemented with --diff flag

### Phase 3: Sync Commands Simplification
- [x] Remove `--include-wmill-yaml` flag - N/A (not present in current version)
- [x] Remove `--settings-from-json` flag - N/A (not present in current version)
- [x] Remove git-sync settings management code - N/A (not present in current version)
- [ ] Update to use `getEffectiveSettings()` - Skipped (requires workspace/repo awareness)
- [x] Copy over JSON output logic from `cli-bak`

### Phase 4: Init Command Update
- [ ] Add git-sync detection logic - Skipped (requires settings commands)
- [ ] Integrate settings pull flow - Skipped (requires settings commands)
- [ ] Support `--repository` flag - Skipped (requires settings commands)
- [x] Updated to use DEFAULT_SYNC_OPTIONS for consistency

### Phase 5: Error Handling Improvements
- [x] Copy network error handling from `cli-bak/context.ts`
- [x] Apply token fix in `requireLogin()`
- [x] Add response body consumption

### Phase 6: Testing and Documentation
- [x] Manual testing of push/pull commands
- [x] Test repository selection (interactive and explicit)
- [x] Test hierarchical settings merging with overrides
- [x] Verify backend integration works correctly
- [ ] Update existing tests
- [ ] Add automated tests for settings merging
- [ ] Add automated tests for repository selection
- [ ] Add automated tests for override scenarios
- [ ] Update CLI documentation
- [ ] Create migration guide
- [ ] Test backward compatibility