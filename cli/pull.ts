// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { colors, Command, JSZip } from "./deps.ts";
import { Workspace } from "./workspace.ts";

export async function downloadZip(
  workspace: Workspace,
  plainSecrets: boolean | undefined
): Promise<JSZip | undefined> {
  const requestHeaders: HeadersInit = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const zipResponse = await fetch(
    workspace.remote +
      "api/w/" +
      workspace.workspaceId +
      "/workspaces/tarball?archive_type=zip&plain_secret=" +
      (plainSecrets ?? false),
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
  }
  const blob = await zipResponse.blob();
  return await JSZip.loadAsync(blob);
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
