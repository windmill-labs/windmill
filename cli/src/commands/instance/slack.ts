import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { getActiveInstance, allInstances, Instance } from "./instance.ts";

async function resolveInstance(
  instanceName: string | undefined
): Promise<Instance> {
  if (instanceName) {
    const match = (await allInstances()).find((i) => i.name === instanceName);
    if (!match) {
      throw new Error(`No local instance profile named ${instanceName}`);
    }
    return match;
  }
  const activeName = await getActiveInstance({});
  if (!activeName) {
    throw new Error(
      "No active instance. Run 'wmill instance add' or pass --instance."
    );
  }
  const match = (await allInstances()).find((i) => i.name === activeName);
  if (!match) {
    throw new Error(`Active instance ${activeName} not found in config`);
  }
  return match;
}

export async function connectSlackInstance(opts: {
  instance?: string;
  botToken: string;
  teamId: string;
  teamName: string;
}) {
  const instance = await resolveInstance(opts.instance);
  setClient(
    instance.token,
    instance.remote.substring(0, instance.remote.length - 1)
  );

  await wmill.connectSlackInstance({
    requestBody: {
      bot_token: opts.botToken,
      team_id: opts.teamId,
      team_name: opts.teamName,
    },
  });

  log.info(
    colors.bold.underline.green(
      `Slack connected at instance ${instance.name} (team ${opts.teamName} / ${opts.teamId})`
    )
  );
}
