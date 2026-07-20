import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";

export async function gitSyncStatus(
  opts: GlobalOptions & { jsonOutput?: boolean },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const mode = await wmill.getGitSyncDeployMode({
    workspace: workspace.workspaceId,
  });

  // `deploy_on_push` means the backend auto-pulls the git remote, so `git push`
  // deploys. Otherwise deploy directly with `wmill sync push`.
  const deployCommand = mode.deploy_on_push ? "git push" : "wmill sync push";

  if (opts.jsonOutput) {
    // console.log (not log.info, which wraps in ANSI color) so the output pipes cleanly to jq.
    console.log(JSON.stringify({ ...mode, deploy_command: deployCommand }, null, 2));
    return;
  }

  if (!mode.configured) {
    log.info(
      colors.yellow(
        "No git-sync repository is configured for this workspace on the backend.",
      ),
    );
  } else if (mode.deploy_on_push) {
    log.info(
      colors.green(
        "Git-sync is set up with auto-pull: pushing to the git remote deploys to this workspace.",
      ),
    );
  } else {
    log.info(
      colors.green(
        "Git-sync is configured, but without auto-pull: deploy with `wmill sync push`.",
      ),
    );
  }
  log.info(`Recommended deploy command: ${colors.bold(deployCommand)}`);
}
