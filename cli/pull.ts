// deno-lint-ignore-file no-explicit-any
import { resolveWorkspace } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import {
  colors,
  Command,
  Confirm,
  copy,
  ensureDir,
  path,
  readerFromStreamReader,
  Untar,
} from "./deps.ts";
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

async function pull(opts: GlobalOptions & { override: boolean }, dir: string) {
  const workspace = await resolveWorkspace(opts);

  const untar = await downloadTar(workspace);
  if (!untar) return;

  for await (const entry of untar) {
    console.log(entry.fileName);
    const filePath = path.resolve(dir, entry.fileName);
    if (entry.type === "directory") {
      await ensureDir(filePath);
      continue;
    }
    await ensureDir(path.dirname(filePath));
    if (!opts.override) {
      let exists = false;
      try {
        const _stat = await Deno.stat(filePath);
        exists = true;
      } catch {
        exists = false;
      }
      if (exists) {
        if (
          !(await Confirm.prompt(
            "Conflict at " +
              filePath +
              " do you want to override the local version?",
          ))
        ) {
          continue;
        }
      }
    }
    const file = await Deno.open(filePath, { write: true, create: true });
    const len = await copy(entry, file);
    await file.truncate(len);
    file.close();
  }
  console.log(colors.green("Done. Wrote all files to disk."));
}

const command = new Command()
  .description(
    "Pull all definitions in the current workspace from the API and write them to disk.",
  )
  .arguments("<dir:string>")
  .action(pull as any);

export default command;
