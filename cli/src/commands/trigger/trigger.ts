import { stat } from "node:fs/promises";

import * as wmill from "../../../gen/services.gen.ts";
import {
  GcpTrigger,
  HttpTrigger,
  KafkaTrigger,
  MqttTrigger,
  NatsTrigger,
  PostgresTrigger,
  SqsTrigger,
  WebsocketTrigger,
  EmailTrigger,
  NativeTrigger,
  NativeTriggerData,
  NativeServiceName,
} from "../../../gen/types.gen.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
  TRIGGER_TYPES,
  extractNativeTriggerInfo,
} from "../../types.ts";
import {
  fromBranchSpecificPath,
  isBranchSpecificFile,
} from "../../core/specific_items.ts";
import { getCurrentGitBranch } from "../../utils/git.ts";
import { requireLogin } from "../../core/auth.ts";
import { validatePath, resolveWorkspace } from "../../core/context.ts";

type Trigger = {
  http: HttpTrigger;
  websocket: WebsocketTrigger;
  kafka: KafkaTrigger;
  nats: NatsTrigger;
  postgres: PostgresTrigger;
  mqtt: MqttTrigger;
  sqs: SqsTrigger;
  gcp: GcpTrigger;
  email: EmailTrigger;
};

type TriggerFile<K extends TriggerType> = Omit<
  Trigger[K],
  | "path"
  | "workspace"
  | "edited_by"
  | "edited_at"
  | "error"
  | "last_server_ping"
  | "server_id"
>;

type TriggerType = keyof Trigger;

async function getTrigger<K extends TriggerType>(
  triggerType: K,
  workspace: string,
  path: string
): Promise<Trigger[K]> {
  const triggerFunctions: {
    [K in TriggerType]: (args: {
      workspace: string;
      path: string;
    }) => Promise<Trigger[K]>;
  } = {
    http: wmill.getHttpTrigger,
    websocket: wmill.getWebsocketTrigger,
    kafka: wmill.getKafkaTrigger,
    nats: wmill.getNatsTrigger,
    postgres: wmill.getPostgresTrigger,
    mqtt: wmill.getMqttTrigger,
    sqs: wmill.getSqsTrigger,
    gcp: wmill.getGcpTrigger,
    email: wmill.getEmailTrigger,
  };
  const triggerFunction = triggerFunctions[triggerType];

  const trigger = await triggerFunction({ workspace, path });
  return trigger;
}

async function updateTrigger<K extends TriggerType>(
  triggerType: K,
  workspace: string,
  path: string,
  trigger: Trigger[K]
): Promise<void> {
  const triggerFunctions: {
    [K in TriggerType]: (args: {
      workspace: string;
      path: string;
      requestBody: Trigger[K];
    }) => Promise<any>;
  } = {
    http: wmill.updateHttpTrigger,
    websocket: wmill.updateWebsocketTrigger,
    kafka: wmill.updateKafkaTrigger,
    nats: wmill.updateNatsTrigger,
    postgres: wmill.updatePostgresTrigger,
    mqtt: wmill.updateMqttTrigger,
    sqs: wmill.updateSqsTrigger,
    gcp: wmill.updateGcpTrigger,
    email: wmill.updateEmailTrigger,
  };
  const triggerFunction = triggerFunctions[triggerType];
  await triggerFunction({ workspace, path, requestBody: trigger });
}

async function createTrigger<K extends TriggerType>(
  triggerType: K,
  workspace: string,
  path: string,
  trigger: Trigger[K]
): Promise<void> {
  const triggerFunctions: {
    [K in TriggerType]: (args: {
      workspace: string;
      path: string;
      requestBody: Trigger[K];
    }) => Promise<any>;
  } = {
    http: wmill.createHttpTrigger,
    websocket: wmill.createWebsocketTrigger,
    kafka: wmill.createKafkaTrigger,
    nats: wmill.createNatsTrigger,
    postgres: wmill.createPostgresTrigger,
    mqtt: wmill.createMqttTrigger,
    sqs: wmill.createSqsTrigger,
    gcp: wmill.createGcpTrigger,
    email: wmill.createEmailTrigger,
  };
  const triggerFunction = triggerFunctions[triggerType];
  await triggerFunction({ workspace, path, requestBody: trigger });
}

export async function pushTrigger<K extends TriggerType>(
  triggerType: K,
  workspace: string,
  path: string,
  trigger: TriggerFile<K> | Trigger[K] | undefined,
  localTrigger: TriggerFile<K>
): Promise<void> {
  path = removeType(path, triggerType + "_trigger").replaceAll(SEP, "/");
  log.debug(`Processing local ${triggerType} trigger ${path}`);

  try {
    trigger = await getTrigger(triggerType, workspace, path);
    log.debug(`${triggerType} trigger ${path} exists on remote`);
  } catch {
    log.debug(`${triggerType} trigger ${path} does not exist on remote`);
    //ignore
  }

  if (trigger) {
    if (isSuperset(localTrigger, trigger)) {
      log.debug(`${triggerType} trigger ${path} is up to date`);
      return;
    }
    log.debug(`${triggerType} trigger ${path} is not up-to-date, updating...`);
    try {
      await updateTrigger(triggerType, workspace, path, {
        ...localTrigger,
        path,
      } as Trigger[K]);
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  } else {
    console.log(
      colors.bold.yellow(`Creating new ${triggerType} trigger: ${path}`)
    );
    try {
      await createTrigger(triggerType, workspace, path, {
        ...localTrigger,
        path,
      } as Trigger[K]);
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  }
}

type NativeTriggerFile = Omit<
  NativeTrigger,
  "external_id" | "workspace_id" | "error"
>;

export async function pushNativeTrigger(
  workspace: string,
  filePath: string,
  _remoteTrigger: NativeTrigger | undefined,
  localTrigger: NativeTriggerFile
): Promise<void> {
  const triggerInfo = extractNativeTriggerInfo(filePath);
  if (!triggerInfo) {
    throw new Error(
      `Invalid native trigger file path: ${filePath}. Expected format: {script_path}.{flow|script}.{external_id}.{service}_native_trigger.json`
    );
  }

  const { externalId, serviceName } = triggerInfo;
  log.debug(
    `Processing local native trigger: service=${serviceName}, external_id=${externalId}`
  );

  let remoteTrigger: NativeTrigger | undefined;
  try {
    const result = await wmill.getNativeTrigger({
      workspace,
      serviceName: serviceName as NativeServiceName,
      externalId,
    });
    // getNativeTrigger returns NativeTriggerWithExternal, extract NativeTrigger fields
    remoteTrigger = {
      external_id: result.external_id,
      workspace_id: result.workspace_id,
      service_name: result.service_name,
      script_path: result.script_path,
      is_flow: result.is_flow,
      service_config: result.service_config,
      error: result.error,
    };
    log.debug(`Native trigger ${serviceName}/${externalId} exists on remote`);
  } catch {
    log.debug(
      `Native trigger ${serviceName}/${externalId} does not exist on remote`
    );
  }

  const triggerData: NativeTriggerData = {
    script_path: localTrigger.script_path,
    is_flow: localTrigger.is_flow,
    service_config: localTrigger.service_config,
  };

  if (remoteTrigger) {
    // Compare relevant fields
    const localCompare = {
      script_path: localTrigger.script_path,
      is_flow: localTrigger.is_flow,
      service_config: localTrigger.service_config,
    };
    const remoteCompare = {
      script_path: remoteTrigger.script_path,
      is_flow: remoteTrigger.is_flow,
      service_config: remoteTrigger.service_config,
    };

    if (isSuperset(localCompare, remoteCompare)) {
      log.debug(`Native trigger ${serviceName}/${externalId} is up to date`);
      return;
    }

    log.debug(
      `Native trigger ${serviceName}/${externalId} is not up-to-date, updating...`
    );
    try {
      await wmill.updateNativeTrigger({
        workspace,
        serviceName: serviceName as NativeServiceName,
        externalId,
        requestBody: triggerData,
      });
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  } else {
    console.log(
      colors.bold.yellow(
        `Creating new native trigger: ${serviceName}/${externalId}`
      )
    );
    try {
      await wmill.createNativeTrigger({
        workspace,
        serviceName: serviceName as NativeServiceName,
        requestBody: triggerData,
      });
    } catch (e) {
      console.error((e as any).body);
      throw e;
    }
  }
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const httpTriggers = await wmill.listHttpTriggers({
    workspace: workspace.workspaceId,
  });
  const websocketTriggers = await wmill.listWebsocketTriggers({
    workspace: workspace.workspaceId,
  });
  const kafkaTriggers = await wmill.listKafkaTriggers({
    workspace: workspace.workspaceId,
  });
  const natsTriggers = await wmill.listNatsTriggers({
    workspace: workspace.workspaceId,
  });
  const postgresTriggers = await wmill.listPostgresTriggers({
    workspace: workspace.workspaceId,
  });
  const mqttTriggers = await wmill.listMqttTriggers({
    workspace: workspace.workspaceId,
  });
  const sqsTriggers = await wmill.listSqsTriggers({
    workspace: workspace.workspaceId,
  });
  const gcpTriggers = await wmill.listGcpTriggers({
    workspace: workspace.workspaceId,
  });
  const emailTriggers = await wmill.listEmailTriggers({
    workspace: workspace.workspaceId,
  });
  const triggers = [
    ...httpTriggers.map((x) => ({ path: x.path, kind: "http" })),
    ...websocketTriggers.map((x) => ({ path: x.path, kind: "websocket" })),
    ...kafkaTriggers.map((x) => ({ path: x.path, kind: "kafka" })),
    ...natsTriggers.map((x) => ({ path: x.path, kind: "nats" })),
    ...postgresTriggers.map((x) => ({ path: x.path, kind: "postgres" })),
    ...mqttTriggers.map((x) => ({ path: x.path, kind: "mqtt" })),
    ...sqsTriggers.map((x) => ({ path: x.path, kind: "sqs" })),
    ...gcpTriggers.map((x) => ({ path: x.path, kind: "gcp" })),
    ...emailTriggers.map((x) => ({ path: x.path, kind: "email" })),
  ];

  new Table()
    .header(["Path", "Kind"])
    .padding(2)
    .border(true)
    .body(triggers.map((x) => [x.path, x.kind]))
    .render();
}

function checkIfValidTrigger(kind: string | undefined): kind is TriggerType {
  if (kind && (TRIGGER_TYPES as readonly string[]).includes(kind)) {
    return true;
  } else {
    return false;
  }
}

function extractTriggerKindFromPath(filePath: string): string | undefined {
  let pathToAnalyze = filePath;

  // If this is a branch-specific file, convert it to the base path first
  if (isBranchSpecificFile(filePath)) {
    const currentBranch = getCurrentGitBranch();
    if (currentBranch) {
      pathToAnalyze = fromBranchSpecificPath(filePath, currentBranch);
    }
  }

  // Now extract trigger type from the base path: "something.kafka_trigger.yaml" -> "kafka"
  const triggerMatch = pathToAnalyze.match(/\.(\w+)_trigger\.yaml$/);
  return triggerMatch ? triggerMatch[1] : undefined;
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing trigger..."));

  const triggerKind = extractTriggerKindFromPath(filePath);
  if (!checkIfValidTrigger(triggerKind)) {
    throw new Error("Invalid trigger kind: " + triggerKind);
  }
  await pushTrigger(
    triggerKind,
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath)
  );
  console.log(colors.bold.underline.green("Trigger pushed"));
}

const command = new Command()
  .description("trigger related commands")
  .action(list as any)
  .command(
    "push",
    "push a local trigger spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
