import { ensureDir } from "https://deno.land/std@0.161.0/fs/ensure_dir.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";
import { getRootStore } from "./store.ts";
import { GlobalOptions } from "./types.ts";

export type Remote = {
  baseUrl: string;
};

export async function getRemote(name: string): Promise<Remote | undefined> {
  const store = (await getRootStore()) + "remotes/";
  await ensureDir(store);
  try {
    return JSON.parse(await Deno.readTextFile(store + name + "/info"));
  } catch {
    return undefined;
  }
}

export async function getDefaultRemote(): Promise<Remote | undefined> {
  const store = await getRootStore();
  try {
    const name = await Deno.readTextFile(store + "/default");
    return JSON.parse(
      await Deno.readTextFile(store + "remotes/" + name + "/info")
    );
  } catch {
    return undefined;
  }
}

export async function setDefault(_opts: GlobalOptions, name: string) {
  const store = await getRootStore();
  try {
    const _info = await Deno.stat(store + "remotes/" + name);
  } catch {
    console.log(colors.red("Remote " + name + " does not exist"));
    return;
  }
  await Deno.writeTextFile(store + "/default", name);
}

export async function add(_opts: GlobalOptions, name: string, baseUrl: string) {
  const store = (await getRootStore()) + "remotes/";
  await ensureDir(store);
  try {
    await Deno.mkdir(store + name);
  } catch {
    console.log(colors.red("This remote already exists"));
    return;
  }
  const remote: Remote = {
    baseUrl,
  };
  await Deno.writeTextFile(store + name + "/info", JSON.stringify(remote));
  console.log(colors.green("Successfully added remote!"));
}

async function remove(_opts: GlobalOptions, name: string) {
  const store = (await getRootStore()) + "remotes/";
  await ensureDir(store);
  try {
    await Deno.remove(store + name, { recursive: true });
  } catch {
    console.log(colors.yellow("This remote doesn't exist."));
  }
  console.log(colors.green("Successfully removed remote!"));
}

async function list(_opts: GlobalOptions) {
  const store = (await getRootStore()) + "remotes/";
  await ensureDir(store);
  const infos: Map<string, Remote> = new Map();
  for await (const e of Deno.readDir(store)) {
    if (!e.isDirectory) continue;
    const name = e.name;
    try {
      infos.set(
        name,
        JSON.parse(await Deno.readTextFile(store + name + "/info"))
      );
    } catch {
      continue;
    }
  }

  new Table()
    .header(["name", "URL"])
    .border(true)
    .padding(2)
    .body(Array.from(infos.entries()).map(([name, r]) => [name, r.baseUrl]))
    .render();
}

const command = new Command()
  .description(
    "remote related commands. Provide no subcommand to list local remotes."
  )
  .action(list as any)
  .command("add", "Add a remote windmill server to interact with.")
  .arguments("<name:string> <base_url:string>")
  .action(add as any)
  .command("remove", "Remove a remote windmill server")
  .arguments("<name:string>")
  .command("set-default", "Set a remote as default")
  .arguments("<name:string>")
  .action(setDefault as any)
  .action(remove as any);

export default command;
