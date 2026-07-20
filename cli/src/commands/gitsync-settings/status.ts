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
  shellQuote,
  stripGitRemoteCredentials,
} from "../../utils/git.ts";

export async function gitSyncStatus(
  opts: GlobalOptions & { jsonOutput?: boolean; remote?: string },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const remoteName = opts.remote ?? "origin";
  const inGitRepo = isGitRepository();
  const branch = inGitRepo ? getCurrentGitBranch() : null;
  const rawRemote = inGitRepo ? getGitRemoteUrl(remoteName) : null;
  // Strip credentials before the URL leaves the machine — it is sent as a query
  // param and would otherwise reach the server's request-URI logs.
  const remote = rawRemote ? stripGitRemoteCredentials(rawRemote) : null;

  // The backend matches this remote+branch against its auto-pull repos and
  // returns whether pushing here deploys — no repo URLs leave the backend.
  const mode = await wmill.getGitSyncDeployMode({
    workspace: workspace.workspaceId,
    remote: remote ?? undefined,
    branch: branch ?? undefined,
  });

  // Only recommend a command when the backend definitively deploys on push:
  // git push the exact remote+branch checked (shell-quoted — this string is
  // agent-executed and names may contain metacharacters). When auto-pull doesn't
  // match, a CI workflow may still deploy on push, so don't assume `wmill sync
  // push`; defer to the Deploying guidance instead.
  const deployCommand = mode.deploy_on_push
    ? `git push ${shellQuote(remoteName)} ${shellQuote(branch ?? "")}`
    : null;

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
        `Git-sync auto-pull tracks this branch and remote: pushing '${branch}' to ${remoteName} deploys to the workspace.`,
      ),
    );
    log.info(`Recommended deploy command: ${colors.bold(deployCommand!)}`);
  } else {
    log.info(
      colors.yellow(
        mode.configured
          ? "Backend auto-pull does not deploy this checkout (branch, remote, or delivery path not matched)."
          : "No git-sync repository is configured for this workspace on the backend.",
      ),
    );
    log.info(
      "A `git push` won't be deployed by the backend here. If a CI workflow deploys on push, use `git push`; otherwise use `wmill sync push`. Record the choice as a `Deploy mode:` line in AGENTS.md so later sessions skip the check (see the Deploying section).",
    );
  }
}
