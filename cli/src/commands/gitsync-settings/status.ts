import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getCurrentGitBranch, isGitRepository } from "../../utils/git.ts";

export async function gitSyncStatus(opts: GlobalOptions & { jsonOutput?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const branch = isGitRepository() ? getCurrentGitBranch() : null;

  // The backend reports whether exactly one licensed, deliverable auto-pull repo
  // tracks this branch — in which case a `git push` deploys. It does not check
  // the remote URL: with a single synced repo the checkout is unambiguously it,
  // and with several the backend returns false so we defer to the user here.
  const mode = await wmill.getGitSyncDeployMode({
    workspace: workspace.workspaceId,
    branch: branch ?? undefined,
  });

  const deployCommand = mode.deploy_on_push ? "git push" : null;

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
        `Git-sync auto-pull deploys this branch: pushing '${branch}' deploys to the workspace.`,
      ),
    );
    log.info(`Recommended deploy command: ${colors.bold("git push")}`);
  } else {
    log.info(
      colors.yellow(
        mode.configured
          ? "Backend auto-pull does not deploy this checkout (no single repo tracks this branch)."
          : "No git-sync repository is configured for this workspace on the backend.",
      ),
    );
    log.info(
      "The backend won't deploy a `git push` here. Ask the user how this repo deploys: `git push` (a CI workflow deploys on push) or `wmill sync push`. Record the answer as a `Deploy mode:` line in AGENTS.md so later sessions skip the question (see the Deploying section).",
    );
  }
}
