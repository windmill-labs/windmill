import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
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
  is_fileset?: boolean;
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

  if (hubBaseUrl !== DEFAULT_HUB_BASE_URL) {
    const hubSecret = (await wmill.getGlobal({
      key: "hub_api_secret",
    })) as string | undefined;
    log.info("Fetching resource types from private hub: " + hubBaseUrl);
    if (hubSecret) {
      log.info("Using hub API secret");
      headers["X-api-secret"] = hubSecret;
    }
  }

  if (uid) {
    headers["X-uid"] = uid;
  }

  let res1 = await fetch(hubBaseUrl + "/resource_types/list", {
    headers,
  });

  if (!res1.ok) {
    if (res1.status === 401) {
      // 401 can only happen on a private hub
      throw new Error("Unauthorized access to private hub: " + hubBaseUrl);
    } else {
      throw new Error(
        "Couldn't fetch resource types from hub " +
          hubBaseUrl +
          ": " +
          (await res1.text())
      );
    }
  }

  let list = (await res1.json()) as HubResourceType[];

  if (list && list.length === 0 && hubBaseUrl !== DEFAULT_HUB_BASE_URL) {
    log.info(
      "No resource types found in private hub, fetching from public hub"
    );
    delete headers["X-api-secret"];
    const res2 = await fetch(DEFAULT_HUB_BASE_URL + "/resource_types/list", {
      headers,
    });

    if (!res2.ok) {
      throw new Error(
        "Couldn't fetch resource types from public hub: " + (await res2.text())
      );
    }

    list = (await res2.json()) as HubResourceType[];
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
          y.description === x.description &&
          (y.is_fileset ?? false) === (x.is_fileset ?? false)
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
