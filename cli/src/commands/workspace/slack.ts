import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";

export async function connectSlack(
  opts: GlobalOptions & {
    botToken: string;
    teamId: string;
    teamName: string;
  }
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);

  await wmill.connectSlack({
    workspace: workspace.workspaceId,
    requestBody: {
      bot_token: opts.botToken,
      team_id: opts.teamId,
      team_name: opts.teamName,
    },
  });

  log.info(
    colors.bold.underline.green(
      `Slack connected to workspace ${workspace.workspaceId} (team ${opts.teamName} / ${opts.teamId})`
    )
  );
}

export async function disconnectSlack(opts: GlobalOptions) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);

  await wmill.disconnectSlack({ workspace: workspace.workspaceId });

  log.info(
    colors.bold.underline.green(
      `Slack disconnected from workspace ${workspace.workspaceId} (slack_team_id / slack_name cleared). ` +
        `To also remove the bot token variable/resource/folder/group, delete the corresponding files from the local sync folder and run 'wmill sync push'. ` +
        `To remove the workspace-level OAuth override (if any), set slack_oauth_client_id/_secret to '' in settings.yaml and push.`
    )
  );
}
