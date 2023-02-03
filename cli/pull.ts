// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { colors, Command, readerFromStreamReader, Untar } from "./deps.ts";
import { Workspace } from "./workspace.ts";

export async function downloadTar(
  workspace: Workspace,
): Promise<Untar | undefined> {
  const requestHeaders: HeadersInit = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const tarResponse = await fetch(
    workspace.remote + "api/w/" + workspace.workspaceId +
      "/workspaces/tarball",
    {
      headers: requestHeaders,
      method: "GET",
    },
  );

  if (!tarResponse.ok) {
    console.log(
      colors.red(
        "Failed to request tarball from API " + tarResponse.statusText,
      ),
    );
    console.log(await tarResponse.text());
    return undefined;
  }

  const streamReader = tarResponse.body?.getReader();
  if (!streamReader) {
    console.log(colors.red("Failed to read tar request body"));
    return undefined;
  }
  console.log(colors.yellow("Streaming tarball to disk..."));
  const denoReader = readerFromStreamReader(streamReader);
  const untar = new Untar(denoReader);
  return untar;
}

async function stub(
  _opts: GlobalOptions & { override: boolean },
  _dir: string,
) {
  console.log(
    colors.red.underline(
      'Pull is deprecated. Use "sync pull --raw" instead. See <TODO_LINK_HERE> for more information.',
    ),
  );
}

const command = new Command()
  .description(
    "Pull all definitions in the current workspace from the API and write them to disk.",
  )
  .arguments("<dir:string>")
  .action(stub as any);

export default command;
