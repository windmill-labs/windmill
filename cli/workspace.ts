/**
 * Barrel file for workspace exports used by tests.
 * Re-exports key functions from the workspace command module.
 */

export { addWorkspace, allWorkspaces, type Workspace } from "./src/commands/workspace/workspace.ts";
