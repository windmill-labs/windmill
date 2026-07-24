/**
 * Shared constants used across the CLI.
 * This file should have minimal imports to avoid circular dependencies.
 */

export const WM_FORK_PREFIX = "wm-fork";

// CLI version — source of truth. Release tooling (.github/change-versions*.sh)
// rewrites this line. Kept here, rather than in main.ts, so low-level modules
// (e.g. utils.ts) can read it without importing main.ts and creating a circular
// dependency (main → workspace → utils → main) that triggers a TDZ.
// Re-exported from main.ts for backwards compatibility.
export const VERSION = "1.769.0";
