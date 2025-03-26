// deno-lint-ignore-file no-explicit-any
import { Command, log } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";

import { requireLogin, resolveWorkspace } from "./context.ts";
import { pushResourceType } from "./resource-type.ts";
import { GlobalOptions } from "./types.ts";
import { deepEqual } from "./utils.ts";

const DEFAULT_HUB_BASE_URL = "https://hub.windmill.dev";

export async function pull(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);

  if (workspace.workspaceId !== "admins") {
    log.info(
      "Should only sync to admins workspace, but current is not admins."
    );
    return;
  }

  const userInfo = await requireLogin(opts);

  const uid = (await wmill.getGlobal({
    key: "uid",
  })) as string;

  const hubBaseUrl =
    (await wmill.getGlobal({
      key: "hub_base_url",
    })) ?? DEFAULT_HUB_BASE_URL;

  const headers: Record<string, string> = {
    Accept: "application/json",
    "X-email": userInfo.email,
  };

  if (uid) {
    headers["X-uid"] = uid;
  }

  let preList = await fetch(hubBaseUrl + "/resource_types/list", {
    headers,
  }).then((r) => r.json() as Promise<{ id: number; name: string }[]>);

  if (preList && preList.length === 0 && hubBaseUrl !== DEFAULT_HUB_BASE_URL) {
    preList = await fetch(DEFAULT_HUB_BASE_URL + "/resource_types/list", {
      headers,
    }).then((r) => r.json() as Promise<{ id: number; name: string }[]>);
  }

  let list: {
    id: number;
    name: string;
    schema: string;
    approved: boolean;
    app: string;
    description: string;
    created_by: string;
    created_at: Date;
    comments: never[];
  }[] = await Promise.all(
    preList.map((x) =>
      fetch(hubBaseUrl + "/resource_types/" + x.id + "/" + x.name, {
        headers: {
          Accept: "application/json",
        },
      })
    )
  )
    .then((x) =>
      x.map((x) =>
        x.json().catch((e) => {
          log.info(e);
          return undefined;
        })
      )
    )
    .then((x) => Promise.all(x))
    .then((x) =>
      (x as { resource_type: any }[])
        .filter((x) => x)
        .map((x) => x.resource_type)
    );

  const resourceTypes = await wmill.listResourceType({
    workspace: workspace.workspaceId,
  });

  for (const x of list) {
    try {
      x.schema = JSON.parse(x.schema);
    } catch (e) {
      log.info("failed to parse schema for " + x.name);
      continue;
    }
    if (
      resourceTypes.find(
        (y) =>
          y.name === x.name &&
          typeof y.schema !== "string" &&
          deepEqual(y.schema, x.schema) &&
          y.description === x.description
      )
    ) {
      log.info("skipping " + x.name + " (same as current)");
      continue;
    }
    log.info("syncing " + x.name);

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
  .description("pull any supported definitions. EXPERIMENTAL.")
  .action(pull as any);

export default command;
