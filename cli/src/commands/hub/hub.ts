// deno-lint-ignore-file no-explicit-any
import { Command, log } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { pushResourceType } from "../resource-type/resource-type.ts";
import { GlobalOptions } from "../../types.ts";
import { deepEqual } from "../../utils/utils.ts";

const DEFAULT_HUB_BASE_URL = "https://hub.windmill.dev";

interface HubResourceType {
  id: number;
  name: string;
  schema: string;
  app: string;
  description: string;
}

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

  let list = await fetch(hubBaseUrl + "/resource_types/list", {
    headers,
  }).then((r) => r.json() as Promise<HubResourceType[]>);

  if (list && list.length === 0 && hubBaseUrl !== DEFAULT_HUB_BASE_URL) {
    list = await fetch(DEFAULT_HUB_BASE_URL + "/resource_types/list", {
      headers,
    }).then((r) => r.json() as Promise<HubResourceType[]>);
  }

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
