import { stat, writeFile } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";

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
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
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

const triggerTemplates: Record<TriggerType, Record<string, any>> = {
  http: {
    script_path: "",
    is_flow: false,
    route_path: "",
    http_method: "get",
    is_async: false,
    requires_auth: true,
  },
  websocket: {
    script_path: "",
    is_flow: false,
    url: "",
    enabled: false,
  },
  kafka: {
    script_path: "",
    is_flow: false,
    kafka_resource_path: "",
    group_id: "",
    topics: [],
    enabled: false,
  },
  nats: {
    script_path: "",
    is_flow: false,
    nats_resource_path: "",
    subjects: [],
    enabled: false,
  },
  postgres: {
    script_path: "",
    is_flow: false,
    postgres_resource_path: "",
    publication_name: "",
    replication_slot_name: "",
    enabled: false,
  },
  mqtt: {
    script_path: "",
    is_flow: false,
    mqtt_resource_path: "",
    topics: [],
    subscribe_qos: 0,
    enabled: false,
  },
  sqs: {
    script_path: "",
    is_flow: false,
    sqs_resource_path: "",
    queue_url: "",
    enabled: false,
  },
  gcp: {
    script_path: "",
    is_flow: false,
    gcp_resource_path: "",
    subscription_id: "",
    topic_id: "",
    enabled: false,
  },
  email: {
    script_path: "",
    is_flow: false,
    enabled: false,
  },
};

async function newTrigger(opts: GlobalOptions & { kind: string }, path: string) {
  if (!validatePath(path)) {
    return;
  }
  if (!opts.kind) {
    throw new Error("--kind is required. Valid kinds: " + TRIGGER_TYPES.join(", "));
  }
  if (!checkIfValidTrigger(opts.kind)) {
    throw new Error("Invalid trigger kind: " + opts.kind + ". Valid kinds: " + TRIGGER_TYPES.join(", "));
  }
  const kind: TriggerType = opts.kind;
  const filePath = `${path}.${kind}_trigger.yaml`;
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template = triggerTemplates[kind];
  await writeFile(filePath, yamlStringify(template), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean; kind?: string }, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (opts.kind) {
    if (!checkIfValidTrigger(opts.kind)) {
      throw new Error("Invalid trigger kind: " + opts.kind + ". Valid kinds: " + TRIGGER_TYPES.join(", "));
    }
    const trigger = await getTrigger(opts.kind, workspace.workspaceId, path);
    if (opts.json) {
      console.log(JSON.stringify(trigger));
    } else {
      console.log(colors.bold("Path:") + " " + (trigger as any).path);
      console.log(colors.bold("Kind:") + " " + opts.kind);
      console.log(colors.bold("Enabled:") + " " + ((trigger as any).enabled ?? "-"));
      console.log(colors.bold("Script Path:") + " " + ((trigger as any).script_path ?? ""));
      console.log(colors.bold("Is Flow:") + " " + ((trigger as any).is_flow ? "true" : "false"));
    }
    return;
  }

  // Try all trigger types and collect matches
  const matches: { kind: string; trigger: any }[] = [];
  for (const kind of TRIGGER_TYPES) {
    try {
      const trigger = await getTrigger(kind, workspace.workspaceId, path);
      matches.push({ kind, trigger });
    } catch {
      // not found for this kind
    }
  }

  if (matches.length === 0) {
    throw new Error("No trigger found at path: " + path);
  }

  if (matches.length === 1) {
    const { kind, trigger } = matches[0];
    if (opts.json) {
      console.log(JSON.stringify(trigger));
    } else {
      console.log(colors.bold("Path:") + " " + trigger.path);
      console.log(colors.bold("Kind:") + " " + kind);
      console.log(colors.bold("Enabled:") + " " + (trigger.enabled ?? "-"));
      console.log(colors.bold("Script Path:") + " " + (trigger.script_path ?? ""));
      console.log(colors.bold("Is Flow:") + " " + (trigger.is_flow ? "true" : "false"));
    }
    return;
  }

  // Multiple matches — ask user to specify --kind
  console.log("Multiple triggers found at path " + path + ":");
  for (const m of matches) {
    console.log("  - " + m.kind);
  }
  console.log("Please specify --kind <type> to select one.");
}

async function listOrEmpty<T>(fn: () => Promise<T[]>): Promise<T[]> {
  try {
    return await fn();
  } catch {
    return [];
  }
}

async function list(opts: GlobalOptions & { json?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const ws = workspace.workspaceId;
  const [
    httpTriggers,
    websocketTriggers,
    kafkaTriggers,
    natsTriggers,
    postgresTriggers,
    mqttTriggers,
    sqsTriggers,
    gcpTriggers,
    emailTriggers,
  ] = await Promise.all([
    listOrEmpty(() => wmill.listHttpTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listWebsocketTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listKafkaTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listNatsTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listPostgresTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listMqttTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listSqsTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listGcpTriggers({ workspace: ws })),
    listOrEmpty(() => wmill.listEmailTriggers({ workspace: ws })),
  ]);
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

  if (opts.json) {
    console.log(JSON.stringify(triggers));
  } else {
    new Table()
      .header(["Path", "Kind"])
      .padding(2)
      .border(true)
      .body(triggers.map((x) => [x.path, x.kind]))
      .render();
  }
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
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all triggers")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a trigger's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .option("--kind <kind:string>", "Trigger kind (http, websocket, kafka, nats, postgres, mqtt, sqs, gcp, email). Recommended for faster lookup")
  .action(get as any)
  .command("new", "create a new trigger locally")
  .arguments("<path:string>")
  .option("--kind <kind:string>", "Trigger kind (required: http, websocket, kafka, nats, postgres, mqtt, sqs, gcp, email)")
  .action(newTrigger as any)
  .command(
    "push",
    "push a local trigger spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
