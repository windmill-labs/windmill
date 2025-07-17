import * as wmill from "./gen/services.gen.ts";
import {
  GcpTrigger,
  HttpTrigger,
  KafkaTrigger,
  MqttTrigger,
  NatsTrigger,
  PostgresTrigger,
  SqsTrigger,
  WebsocketTrigger,
} from "./gen/types.gen.ts";
import { colors, Command, log, SEP, Table } from "./deps.ts";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import { requireLogin } from "./auth.ts";
import { validatePath, resolveWorkspace } from "./context.ts";

type Trigger = {
  http: HttpTrigger;
  websocket: WebsocketTrigger;
  kafka: KafkaTrigger;
  nats: NatsTrigger;
  postgres: PostgresTrigger;
  mqtt: MqttTrigger;
  sqs: SqsTrigger;
  gcp: GcpTrigger;
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
    gcp: async (args) => {
      throw new Error("GCP triggers are not supported yet");
    },
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
    gcp: async (args) => {
      throw new Error("GCP triggers are not supported yet");
    },
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
      console.error(e.body);
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
      console.error(e.body);
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
  const triggers = [
    ...httpTriggers.map((x) => ({ path: x.path, kind: "http" })),
    ...websocketTriggers.map((x) => ({ path: x.path, kind: "websocket" })),
    ...kafkaTriggers.map((x) => ({ path: x.path, kind: "kafka" })),
    ...natsTriggers.map((x) => ({ path: x.path, kind: "nats" })),
    ...postgresTriggers.map((x) => ({ path: x.path, kind: "postgres" })),
    ...mqttTriggers.map((x) => ({ path: x.path, kind: "mqtt" })),
    ...sqsTriggers.map((x) => ({ path: x.path, kind: "sqs" })),
    ...gcpTriggers.map((x) => ({ path: x.path, kind: "gcp" })),
  ];

  new Table()
    .header(["Path", "Kind"])
    .padding(2)
    .border(true)
    .body(triggers.map((x) => [x.path, x.kind]))
    .render();
}

function checkIfValidTrigger(kind: string | undefined): kind is TriggerType {
  if (
    kind &&
    [
      "http",
      "websocket",
      "kafka",
      "nats",
      "postgres",
      "mqtt",
      "sqs",
      "gcp",
    ].includes(kind)
  ) {
    return true;
  } else {
    return false;
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing trigger..."));

  const triggerKind = filePath.split(".")[1].split("_")[0];
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
