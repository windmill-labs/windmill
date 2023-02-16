// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { colors, Command, zip } from "./deps.ts";
import { Workspace } from "./workspace.ts";

export async function downloadZip(
  workspace: Workspace,
): Promise<string | undefined> {
  const requestHeaders: HeadersInit = new Headers();
  requestHeaders.set("Authorization", "Bearer " + workspace.token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const zipResponse = await fetch(
    workspace.remote + "api/w/" + workspace.workspaceId +
      "/workspaces/tarball?archive_type=zip",
    {
      headers: requestHeaders,
      method: "GET",
    },
  );

  if (!zipResponse.ok) {
    console.log(
      colors.red(
        "Failed to request tarball from API " + zipResponse.statusText,
      ),
    );
    console.log(await zipResponse.text());
    return undefined;
  }
  const tempFilePath = await Deno.makeTempFile();
  const tempFile = await Deno.open(tempFilePath, {
    write: true,
  });
  await zipResponse.body?.pipeTo(tempFile.writable);
  const dir = await Deno.makeTempDir();
  await zip.decompress(tempFilePath, dir);
  // await Deno.remove(tempFilePath);
  return dir;
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
