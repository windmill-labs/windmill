import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { allInstances, getActiveInstance, InstanceSyncOptions, pickInstance } from "../instance/instance.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { pullInstanceConfigs, pushInstanceConfigs } from "../../core/settings.ts";

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

export function removeWorkerPrefix(name: string) {
  if (name.startsWith("worker__")) {
    return name.substring(8);
  }
  return name;
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
        .body(wGroups.map((x) => [removeWorkerPrefix(x.name), JSON.stringify(x.config, null, 2)]))
        .render();
    } else {
      log.error(`Instance ${activeInstance} not found`);
    }
  } else {
    log.info("No active instance found");
    log.info("Use 'wmill instance add' to add a new instance");
  }
}
async function pullWorkerGroups(opts: InstanceSyncOptions) {
  await pickInstance(opts, true);

  const totalChanges =  await pullInstanceConfigs(opts, true) ?? 0;

  if (totalChanges === 0) {
    log.info("No changes to apply");
    return;
  }

  let confirm = true;
  if (opts.yes !== true) {
    confirm = await Confirm.prompt({
      message: `Do you want to pul these ${totalChanges} instance-level changes?`,
      default: true,
    });
  }

  if (confirm) {
    await pullInstanceConfigs(opts, false);
  }
}

async function pushWorkerGroups(opts: InstanceSyncOptions) {
  await pickInstance(opts, true);

  const totalChanges = await pushInstanceConfigs(opts, true) ?? 0;

  if (totalChanges === 0) {
    log.info("No changes to apply");
    return;
  }

  let confirm = true;
  if (opts.yes !== true) {
    confirm = await Confirm.prompt({
      message: `Do you want to apply these ${totalChanges} instance-level changes?`,
      default: true,
    });
  }

  if (confirm) {
    await pushInstanceConfigs(opts, false);
  }
}



const command = new Command()
  .description("display worker groups, pull and push worker groups configs")
  .action(displayWorkerGroups)
  .command("pull")
  .description(
    "Pull worker groups (similar to `wmill instance pull --skip-users --skip-settings --skip-groups`)"
  )
  .option(
    "--instance",
    "Name of the instance to push to, override the active instance"
  )
  .option(
    "--base-url",
    "Base url to be passed to the instance settings instead of the local one"
  )
  .option("--yes", "Pull without needing confirmation")
  .action(pullWorkerGroups as any)
  .command("push")
  .description(
    "Push instance settings, users, configs, group and overwrite remote"
  )
  .option(
    "--instance [instance]",
    "Name of the instance to push to, override the active instance"
  )
  .option(
    "--base-url [baseUrl]",
    "If used with --token, will be used as the base url for the instance"
  )
  .option("--yes", "Push without needing confirmation")
  .action(pushWorkerGroups as any);



export default command;
