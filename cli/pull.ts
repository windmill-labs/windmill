// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { colors, Command, JSZip, log } from "./deps.ts";
import { Workspace } from "./workspace.ts";
import { getHeaders } from "./utils.ts";

export async function downloadZip(
  workspace: Workspace,
  plainSecrets: boolean | undefined,
  skipVariables?: boolean,
  skipResources?: boolean,
  skipSecrets?: boolean,
  includeSchedules?: boolean,
  defaultTs?: "bun" | "deno"
): Promise<JSZip | undefined> {
  const requestHeaders: HeadersInit = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      requestHeaders.set(key, value);
    }
  }

  const zipResponse = await fetch(
    workspace.remote +
      "api/w/" +
      workspace.workspaceId +
      `/workspaces/tarball?archive_type=zip&plain_secret=${
        plainSecrets ?? false
      }&skip_variables=${skipVariables ?? false}&skip_resources=${
        skipResources ?? false
      }&skip_secrets=${skipSecrets ?? false}&include_schedules=${
        includeSchedules ?? false
      }&default_ts=${defaultTs ?? "deno"}`,
    {
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
  return await JSZip.loadAsync(blob as any);
}

async function stub(
  _opts: GlobalOptions & { override: boolean },
  _dir: string
) {
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
