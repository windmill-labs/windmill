import { writeFileSync } from "node:fs";
import { stat, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { stringify as yamlStringify } from "yaml";

import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { ResourceType } from "../../../gen/types.gen.ts";
import { compileResourceTypeToTsType } from "../../utils/resource_types.ts";
import { capitalize, toCamel } from "../../utils/utils.ts";

export interface ResourceTypeFile {
  schema?: any;
  description?: string;
  is_fileset?: boolean;
}

export async function pushResourceType(
  workspace: string,
  remotePath: string,
  resource: ResourceTypeFile | ResourceType | undefined,
  localResource: ResourceTypeFile
): Promise<void> {
  remotePath = removeType(remotePath, "resource-type");
  try {
    resource = await wmill.getResourceType({
      workspace: workspace,
      path: remotePath,
    });
  } catch {
    // resource type doesn't exist
  }

  if (resource) {
    if (isSuperset(localResource, resource)) {
      return;
    }

    await wmill.updateResourceType({
      workspace: workspace,
      path: remotePath,
      requestBody: {
        ...localResource,
      },
    });
  } else {
    log.info(colors.yellow.bold("Creating new resource type..."));
    await wmill.createResourceType({
      workspace: workspace,
      requestBody: {
        name: remotePath,
        ...localResource,
      },
    });
  }
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, name: string) {
  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  log.info(colors.bold.yellow("Pushing resource..."));

  await pushResourceType(
    workspace.workspaceId,
    name,
    undefined,
    parseFromFile(filePath)
  );
  log.info(colors.bold.underline.green("Resource pushed"));
}

async function list(opts: GlobalOptions & { schema?: boolean; json?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const res = await wmill.listResourceType({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(res));
  } else if (opts.schema) {
    new Table()
      .header(["Workspace", "Name", "Schema"])
      .padding(2)
      .border(true)
      .body(
        res.map((x) => [
          x.workspace_id ?? "Global",
          x.name,
          JSON.stringify(x.schema, null, 2),
        ])
      )
      .render();
  } else {
    new Table()
      .header(["Workspace", "Name"])
      .padding(2)
      .border(true)
      .body(res.map((x) => [x.workspace_id ?? "Global", x.name]))
      .render();
  }
}

async function newResourceType(opts: GlobalOptions, name: string) {
  const filePath = name + ".resource-type.yaml";
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: ResourceTypeFile = {
    schema: {},
    description: "",
  };
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const rt = await wmill.getResourceType({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(rt));
  } else {
    console.log(colors.bold("Name:") + " " + rt.name);
    console.log(colors.bold("Description:") + " " + (rt.description ?? ""));
    console.log(colors.bold("Workspace:") + " " + (rt.workspace_id ?? "Global"));
    if (rt.schema) {
      console.log(colors.bold("Schema:") + " " + JSON.stringify(rt.schema, null, 2));
    }
  }
}

export async function generateRTNamespace(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const rts = await wmill.listResourceType({
    workspace: workspace.workspaceId,
  });

  let namespaceContent = "declare namespace RT {\n";
  namespaceContent += rts
    .map((resourceType) => {
      return `  type ${toCamel(
        capitalize(resourceType.name)
      )} = ${compileResourceTypeToTsType(resourceType.schema as any).replaceAll(
        "\n",
        "\n  "
      )}`;
    })
    .join("\n\n");
  namespaceContent += "\n}";

  writeFileSync(path.join(process.cwd(), "rt.d.ts"), namespaceContent);

  log.info(
    colors.green(
      "Created rt.d.ts with resource types namespace (RT) for TypeScript."
    )
  );
}

const command = new Command()
  .description("resource type related commands")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all resource types")
  .option("--schema", "Show schema in the output")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a resource type's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("new", "create a new resource type locally")
  .arguments("<name:string>")
  .action(newResourceType as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <name:string>")
  .action(push as any)
  .command(
    "generate-namespace",
    "Create a TypeScript definition file with the RT namespace generated from the resource types"
  )
  .action(generateRTNamespace as any);

export default command;
