import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getCurrentGitBranch, isGitRepository } from "../../utils/git.ts";

export async function gitSyncStatus(
  opts: GlobalOptions & { jsonOutput?: boolean },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const mode = await wmill.getGitSyncDeployMode({
    workspace: workspace.workspaceId,
  });

  const currentBranch = isGitRepository() ? getCurrentGitBranch() : null;

  // Auto-pull deploys only the branches it tracks, so `git push` deploys the
  // current checkout only when its branch is one of those.
  const deploysCurrentBranch =
    mode.deploy_on_push &&
    currentBranch != null &&
    mode.auto_pull_branches.includes(currentBranch);
  const deployCommand = deploysCurrentBranch ? "git push" : "wmill sync push";

  if (opts.jsonOutput) {
    // console.log (not log.info, which wraps in ANSI color) so the output pipes cleanly to jq.
    console.log(
      JSON.stringify(
        { ...mode, current_branch: currentBranch, deploy_command: deployCommand },
        null,
        2,
      ),
    );
    return;
  }

  if (!mode.configured) {
    log.info(
      colors.yellow(
        "No git-sync repository is configured for this workspace on the backend.",
      ),
    );
  } else if (deploysCurrentBranch) {
    log.info(
      colors.green(
        `Git-sync auto-pull tracks '${currentBranch}': pushing this branch deploys to the workspace.`,
      ),
    );
  } else if (mode.deploy_on_push) {
    const branches = mode.auto_pull_branches.join(", ") || "(none resolved)";
    log.info(
      colors.yellow(
        `Git-sync auto-pull is enabled for branches [${branches}], but not the current branch${
          currentBranch ? ` '${currentBranch}'` : ""
        }: deploy with \`wmill sync push\`.`,
      ),
    );
  } else {
    log.info(
      colors.green(
        "Git-sync is configured without deploy-on-push: deploy with `wmill sync push`.",
      ),
    );
  }
  log.info(`Recommended deploy command: ${colors.bold(deployCommand)}`);
}
