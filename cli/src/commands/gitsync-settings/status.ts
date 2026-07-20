import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  getCurrentGitBranch,
  getGitRemoteUrl,
  isGitRepository,
} from "../../utils/git.ts";

export async function gitSyncStatus(
  opts: GlobalOptions & { jsonOutput?: boolean; remote?: string },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const inGitRepo = isGitRepository();
  const branch = inGitRepo ? getCurrentGitBranch() : null;
  const remote = inGitRepo ? getGitRemoteUrl(opts.remote) : null;

  // The backend matches this remote+branch against its auto-pull repos and
  // returns whether pushing here deploys — no repo URLs leave the backend.
  const mode = await wmill.getGitSyncDeployMode({
    workspace: workspace.workspaceId,
    remote: remote ?? undefined,
    branch: branch ?? undefined,
  });

  const deployCommand = mode.deploy_on_push ? "git push" : "wmill sync push";

  if (opts.jsonOutput) {
    // console.log (not log.info, which wraps in ANSI color) so the output pipes cleanly to jq.
    console.log(
      JSON.stringify(
        { ...mode, current_branch: branch, deploy_command: deployCommand },
        null,
        2,
      ),
    );
    return;
  }

  if (mode.deploy_on_push) {
    log.info(
      colors.green(
        `Git-sync auto-pull tracks this branch and remote: pushing '${branch}' deploys to the workspace.`,
      ),
    );
  } else if (!mode.configured) {
    log.info(
      colors.yellow(
        "No git-sync repository is configured for this workspace on the backend.",
      ),
    );
  } else {
    log.info(
      colors.yellow(
        "Pushing the current checkout does not deploy (no auto-pull match): deploy with `wmill sync push`.",
      ),
    );
  }
  log.info(`Recommended deploy command: ${colors.bold(deployCommand)}`);
}
