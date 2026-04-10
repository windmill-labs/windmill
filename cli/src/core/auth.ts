import { colors } from "@cliffy/ansi/colors";
import * as log from "./log.ts";
import { setClient } from "./client.ts";
import * as wmill from "../../gen/services.gen.ts";
import { GlobalUserInfo, User } from "../../gen/types.gen.ts";
import { ApiError } from "../../gen/core/ApiError.ts";

import { loginInteractive, tryGetLoginInfo } from "./login.ts";
import { GlobalOptions } from "../types.ts";


// Workspace-scoped tokens (tokens bound to a workspace via token.workspace_id)
// cannot call /api/users/whoami because the backend rejects any token with a
// workspace binding when the request path has no workspace in it. Fall back to
// the workspace-scoped whoami endpoint in that case so the CLI still works.
async function fetchWhoami(workspaceId: string): Promise<GlobalUserInfo> {
  try {
    return await wmill.globalWhoami();
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      const user = await wmill.whoami({ workspace: workspaceId });
      return workspaceUserToGlobalUserInfo(user);
    }
    throw error;
  }
}

function workspaceUserToGlobalUserInfo(user: User): GlobalUserInfo {
  return {
    email: user.email,
    login_type: "password",
    super_admin: user.is_super_admin,
    verified: true,
    name: user.name,
    username: user.username,
    operator_only: user.operator,
    first_time_user: false,
    role_source: "manual",
    disabled: user.disabled,
  };
}

/**
 * Main authentication function - moved from context.ts to break circular dependencies
 * This function maintains the original API signature from context.ts
 */
export async function requireLogin(
  opts: GlobalOptions
): Promise<GlobalUserInfo> {
  // Import resolveWorkspace to avoid circular dependency at module level
  const { resolveWorkspace } = await import("./context.ts");
  const workspace = await resolveWorkspace(opts);

  let token = await tryGetLoginInfo(opts);

  if (!token) {
    token = workspace.token;
  }

  setClient(token, workspace.remote.substring(0, workspace.remote.length - 1));

  try {
    return await fetchWhoami(workspace.workspaceId);
  } catch (error) {
    // Check for network errors and provide clearer messages
    const errorMsg = error instanceof Error ? error.message : String(error);
    if (errorMsg.includes('fetch') || errorMsg.includes('connection') || errorMsg.includes('ECONNREFUSED') || errorMsg.includes('refused')) {
      throw new Error(`Network error: Could not connect to Windmill server at ${workspace.remote}`);
    }

    // If the user explicitly provided credentials via flags, fail immediately
    // rather than falling back to interactive login — they expect their explicit
    // credentials to work and should fix them if they don't.
    if (opts.token || opts.baseUrl) {
      log.info(colors.red("Could not authenticate with the provided credentials. Please check your --token and --base-url and try again."));
      return process.exit(1);
    }

    log.info(
      "! Could not reach API given existing credentials. Attempting to reauth..."
    );
    const newToken = await loginInteractive(workspace.remote);
    if (!newToken) {
      throw new Error("Unauthorized: Could not authenticate with the provided credentials");
    }

    // Update workspace token
    const { removeWorkspace, addWorkspace } = await import("../commands/workspace/workspace.ts");
    removeWorkspace(workspace.name, false, opts);
    workspace.token = newToken;
    addWorkspace(workspace, opts);

    setClient(
      newToken,
      workspace.remote.substring(0, workspace.remote.length - 1)
    );
    return await fetchWhoami(workspace.workspaceId);
  }
}
