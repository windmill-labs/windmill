// deno-lint-ignore-file no-explicit-any
import { Untar } from "https://deno.land/std@0.162.0/archive/tar.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  copy,
  readerFromStreamReader,
} from "https://deno.land/std@0.162.0/streams/mod.ts";
import { resolveWorkspace } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import * as path from "https://deno.land/std@0.162.0/path/mod.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { ensureDir } from "https://deno.land/std@0.162.0/fs/ensure_dir.ts";
import { Confirm } from "https://deno.land/x/cliffy@v0.25.4/prompt/confirm.ts";

async function pull(opts: GlobalOptions & { override: boolean }, dir: string) {
  const workspace = await resolveWorkspace(opts);

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
    console.log(colors.red("Failed to request tarball from API"));
    console.log(colors.red(await tarResponse.text()));
    return;
  }

  const streamReader = tarResponse.body?.getReader();
  if (!streamReader) {
    console.log(colors.red("Failed to read tar request body"));
    return;
  }
  console.log(colors.yellow("Streaming tarball to disk..."));
  const denoReader = readerFromStreamReader(streamReader);
  const untar = new Untar(denoReader);
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
    await copy(entry, file);
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
