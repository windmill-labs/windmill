import { colors } from "@cliffy/ansi/colors";
import * as log from "./log.ts";
import { setClient } from "./client.ts";
import * as wmill from "../../gen/services.gen.ts";
import { GlobalUserInfo } from "../../gen/types.gen.ts";

import { loginInteractive, tryGetLoginInfo } from "./login.ts";
import { GlobalOptions } from "../types.ts";

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
    return await wmill.globalWhoami();
  } catch (error) {
    // Check for network errors and provide clearer messages
    const errorMsg = error instanceof Error ? error.message : String(error);
    if (errorMsg.includes('fetch') || errorMsg.includes('connection') || errorMsg.includes('ECONNREFUSED') || errorMsg.includes('refused')) {
      throw new Error(`Network error: Could not connect to Windmill server at ${workspace.remote}`);
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
    return await wmill.globalWhoami();
  }
}
