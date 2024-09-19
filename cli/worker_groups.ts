import { Command, setClient, Table } from "./deps.ts";

import { log } from "./deps.ts";
import { allInstances, getActiveInstance, Instance } from "./instance.ts";
import * as wmill from "./gen/services.gen.ts";

type GlobalOptions = {
  instance?: string;
  baseUrl?: string;
};
export async function getInstance(opts: GlobalOptions) {
  const instances = await allInstances();

  const instanceName = await getActiveInstance(opts);
  const instance = instances.find((i) => i.name === instanceName);
  if (instance) {
    setClient(
      instance.token,
      instance.remote.slice(0, instance.remote.length - 1)
    );
  }
  return instance;
}

export async function displayWorkerGroups(opts: void) {
  log.info("2 actions available, pull and push.");
  const activeInstance = await getActiveInstance({});

  if (activeInstance) {
    log.info("Active instance: " + activeInstance);
    const instance = await getInstance({});
    if (instance) {
      const wGroups = await wmill.listWorkerGroups();
      new Table()
        .header(["name", "config"])
        .padding(2)
        .border(true)
        .body(wGroups.map((x) => [x.name, JSON.stringify(x.config, null, 2)]))
        .render();
    } else {
      log.error(`Instance ${activeInstance} not found`);
    }
  } else {
    log.info("No active instance found");
    log.info("Use 'wmill instance add' to add a new instance");
  }
}
const command = new Command()
  .description("display worker groups, pull and push worker groups configs")
  .action(displayWorkerGroups)
  .command("pull")
  .description(
    "Pull worker groups (similar to `wmill instance pull --skip-users --skip-settings --skip-groups`)"
  )
  .action(async () => {
    console.log("pulling");
  })
  .command("push")
  .description(
    "Push instance settings, users, configs, group and overwrite remote"
  )
  .option(
    "--instance",
    "Name of the instance to push to, override the active instance"
  )
  .option(
    "--base-url",
    "Base url to be passed to the instance settings instead of the local one"
  )
  .action(async () => {
    console.log("pushing");
  });

export default command;
