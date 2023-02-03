import { Command } from "./deps.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { ResourceTypeFile } from "./resource-type.ts";
import { GlobalOptions } from "./types.ts";

async function pull(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);

  if (workspace.workspaceId !== "admins") {
    console.log(
      "Should only sync to admins workspace, but current is not admins.",
    );
    return;
  }

  await requireLogin(opts);
  const list: {
    id: number;
    name: string;
    schema: string;
    approved: boolean;
    app: string;
    description: string;
    created_by: string;
    created_at: Date;
    comments: never[];
  }[] = await fetch(
    "https://hub.windmill.dev/resource_types/list",
  )
    .then((r) => r.json())
    .then((list: { id: number; name: string }[]) =>
      list.map((x) =>
        fetch(
          "https://hub.windmill.dev/resource_types/" + x.id + "/" + x.name,
          {
            headers: {
              "Accept": "application/json",
            },
          },
        )
      )
    )
    .then((x) => Promise.all(x))
    .then((x) =>
      x.map((x) =>
        x.json().catch((e) => {
          console.log(e);
          return undefined;
        })
      )
    )
    .then((x) => Promise.all(x))
    .then((x) => x.filter((x) => x).map((x) => x.resource_type));

  for (
    const x of list
  ) {
    console.log("syncing " + x.name);
    const f = new ResourceTypeFile();
    f.description = x.description;
    f.schema = JSON.parse(x.schema);
    await f.push(workspace.workspaceId, x.name);
  }
}

const command = new Command()
  .name("hub")
  .description("Hub related commands. EXPERIMENTAL. INTERNAL USE ONLY.")
  .command("pull")
  .description("pull any supported defintions. EXPERIMENTAL.")
  .action(pull as any);

export default command;
