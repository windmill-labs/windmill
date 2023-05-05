// deno-lint-ignore-file no-explicit-any
import { Command, ResourceService } from "./deps.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { pushResourceType } from "./resource-type.ts";
import { GlobalOptions } from "./types.ts";

async function pull(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);

  if (workspace.workspaceId !== "admins") {
    console.log(
      "Should only sync to admins workspace, but current is not admins."
    );
    return;
  }

  const userInfo = await requireLogin(opts);
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
  }[] = await fetch("https://hub.windmill.dev/resource_types/list", {
    headers: {
      Accept: "application/json",
      "X-email": userInfo.email,
    },
  })
    .then((r) => r.json())
    .then((list: { id: number; name: string }[]) =>
      list.map((x) =>
        fetch(
          "https://hub.windmill.dev/resource_types/" + x.id + "/" + x.name,
          {
            headers: {
              Accept: "application/json",
            },
          }
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

  const resourceTypes = await ResourceService.listResourceType({
    workspace: workspace.workspaceId,
  });

  for (const x of list) {
    if (resourceTypes.find((y) => y.name === x.name)) {
      console.log("skipping " + x.name);
      continue;
    }
    console.log("syncing " + x.name);
    await pushResourceType(
      workspace.workspaceId,
      x.name + ".resource-type.json",
      undefined,
      x
    );
  }
}

const command = new Command()
  .name("hub")
  .description("Hub related commands. EXPERIMENTAL. INTERNAL USE ONLY.")
  .command("pull")
  .description("pull any supported defintions. EXPERIMENTAL.")
  .action(pull as any);

export default command;
