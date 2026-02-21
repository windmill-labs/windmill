import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "@std/log";
import JSZip from "jszip";
import { Workspace } from "../workspace/workspace.ts";
import { getHeaders } from "../../utils/utils.ts";

export async function downloadZip(
  workspace: Workspace,
  plainSecrets: boolean | undefined,
  skipVariables?: boolean,
  skipResources?: boolean,
  skipResourceTypes?: boolean,
  skipSecrets?: boolean,
  includeSchedules?: boolean,
  includeTriggers?: boolean,
  includeUsers?: boolean,
  includeGroups?: boolean,
  includeSettings?: boolean,
  includeKey?: boolean,
  skipWorkspaceDependencies?: boolean,
  defaultTs?: "bun" | "deno"
): Promise<JSZip | undefined> {
  const requestHeaders = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      requestHeaders.set(key, value);
    }
  }

  const includeWorkspaceDependenciesValue = !(skipWorkspaceDependencies ?? false);
  const url = workspace.remote +
    "api/w/" +
    workspace.workspaceId +
    `/workspaces/tarball?archive_type=zip&plain_secret=${plainSecrets ?? false
    }&skip_variables=${skipVariables ?? false}&skip_resources=${skipResources ?? false
    }&skip_secrets=${skipSecrets ?? false}&include_schedules=${includeSchedules ?? false
    }&include_triggers=${includeTriggers ?? false}&include_users=${includeUsers ?? false
    }&include_groups=${includeGroups ?? false}&include_settings=${includeSettings ?? false
    }&include_key=${includeKey ?? false}&include_workspace_dependencies=${includeWorkspaceDependenciesValue}&default_ts=${defaultTs ?? "bun"}&skip_resource_types=${skipResourceTypes ?? false}&settings_version=v2`;

  const zipResponse = await fetch(url, {
      headers: requestHeaders,
      method: "GET",
    }
  );

  if (!zipResponse.ok) {
    console.log(
      colors.red("Failed to request tarball from API " + zipResponse.statusText)
    );
    throw new Error(await zipResponse.text());
  } else {
    log.debug(`Downloaded zip/tarball successfully`);
  }
  const blob = await zipResponse.blob();
  return await JSZip.loadAsync((await blob.arrayBuffer()) as any);
}

function stub(_opts: GlobalOptions & { override: boolean }, _dir: string) {
  console.log(
    colors.red.underline(
      'Pull is deprecated. Use "sync pull --raw" instead. See <TODO_LINK_HERE> for more information.'
    )
  );
}

const command = new Command()
  .description(
    "Pull all definitions in the current workspace from the API and write them to disk."
  )
  .arguments("<dir:string>")
  .action(stub as any);

export default command;
