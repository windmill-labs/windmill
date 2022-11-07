import { Untar } from "https://deno.land/std@0.162.0/archive/tar.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  copy,
  readerFromStreamReader,
} from "https://deno.land/std@0.162.0/streams/mod.ts";
import { getContext } from "./context.ts";
import { GlobalOptions } from "./types.ts";
import * as path from "https://deno.land/std@0.162.0/path/mod.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { ensureDir } from "https://deno.land/std@0.162.0/fs/ensure_dir.ts";

async function pull(opts: GlobalOptions, dir: string) {
  const { workspace, baseUrl, token } = await getContext(opts);

  const requestHeaders: HeadersInit = new Headers();
  requestHeaders.set("Authorization", "Bearer " + token);
  requestHeaders.set("Content-Type", "application/octet-stream");

  const tarResponse = await fetch(
    baseUrl + "/api/w/" + workspace + "/workspaces/tarball",
    {
      headers: requestHeaders,
      method: "GET",
    }
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
    const file = await Deno.open(filePath, { write: true, create: true });
    await copy(entry, file);
    file.close();
  }
  console.log(colors.green("Done. Wrote all files to disk."));
}

const command = new Command()
  .description("tarball related commands")
  .command("pull")
  .description(
    "Download the tarball of the current workspace & unpack into a directory"
  )
  .arguments("<dir:string>")
  .action(pull as any);

export default command;
