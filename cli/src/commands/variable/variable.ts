import { mkdir, stat, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { stringify as yamlStringify } from "yaml";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";

import * as wmill from "../../../gen/services.gen.ts";
import { ListableVariable } from "../../../gen/types.gen.ts";

async function list(opts: GlobalOptions & { json?: boolean }) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const variables = await wmill.listVariable({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(variables));
  } else {
    new Table()
      .header(["Path", "Is Secret", "Account", "Value"])
      .padding(2)
      .border(true)
      .body(
        variables.map((x) => [
          x.path,
          x.is_secret ? "true" : "false",
          x.account ?? "-",
          x.value ?? "-",
        ])
      )
      .render();
  }
}

async function newVariable(opts: GlobalOptions, path: string) {
  if (!validatePath(path)) {
    return;
  }
  const filePath = path + ".variable.yaml";
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: VariableFile = {
    value: "",
    is_secret: false,
    description: "",
  };
  await mkdir(dirname(filePath), { recursive: true });
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const v = await wmill.getVariable({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(v));
  } else {
    console.log(colors.bold("Path:") + " " + v.path);
    console.log(colors.bold("Value:") + " " + (v.value ?? "-"));
    console.log(colors.bold("Is Secret:") + " " + (v.is_secret ? "true" : "false"));
    console.log(colors.bold("Description:") + " " + (v.description ?? ""));
    console.log(colors.bold("Account:") + " " + (v.account ?? "-"));
  }
}

export interface VariableFile {
  value: string;
  is_secret: boolean;
  description: string;
  account?: number;
  is_oauth?: boolean;
}

/**
 * Whether `value` has the structural shape of a workspace-encrypted secret
 * (the form produced by `sync pull` without --plain-secrets), as opposed to a
 * plaintext value a user authored by hand.
 *
 * Mirrors the server guard (windmill-store/src/variables.rs): workspace
 * ciphertext (AES-256-CBC, base64) is standard base64 decoding to a non-zero
 * multiple of the 16-byte block size. External secret-backend markers
 * ($vault:/$aws_sm:/$azure_kv:) are stored verbatim too, so they count as
 * already-encrypted. This is a shape check only — it never decrypts.
 */
export function looksLikeWorkspaceCiphertext(value: string): boolean {
  if (
    value.startsWith("$vault:") ||
    value.startsWith("$aws_sm:") ||
    value.startsWith("$azure_kv:")
  ) {
    return true;
  }
  if (value.length === 0 || value.length % 4 !== 0) return false;
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(value)) return false;
  const decodedLen = Buffer.from(value, "base64").length;
  return decodedLen > 0 && decodedLen % 16 === 0;
}

export async function pushVariable(
  workspace: string,
  remotePath: string,
  variable: VariableFile | ListableVariable | undefined,
  localVariable: VariableFile,
  plainSecrets: boolean,
  wsSpecific?: boolean,
  // Whether a secret->non-secret downgrade may be applied. Only an authoritative
  // single-file `variable push` sets this. Bulk `sync push` leaves it false: a
  // pulled secret's spec value is ciphertext, and demoting it would store that
  // ciphertext verbatim as a visible non-secret value.
  allowSecretDowngrade: boolean = false,
): Promise<void> {
  remotePath = removeType(remotePath, "variable");
  log.debug(`Processing local variable ${remotePath}`);

  try {
    variable = await wmill.getVariable({
      workspace: workspace,
      path: remotePath.replaceAll(SEP, "/"),
      decryptSecret: plainSecrets,
      includeEncrypted: true,
    });
    log.debug(`Variable ${remotePath} exists on remote`);
  } catch {
    log.debug(`Variable ${remotePath} does not exist on remote`);
  }

  if (variable) {
    if (isSuperset(localVariable, variable)) {
      log.debug(`Variable ${remotePath} is up-to-date`);
      return;
    }

    log.debug(`Variable ${remotePath} is not up-to-date, updating`);

    // Apply is_secret only when it differs from the remote (the value is always
    // sent, so the server allows the flag change). Upgrades (non-secret->secret)
    // always apply; downgrades only when explicitly allowed (single-file push) —
    // see allowSecretDowngrade. `undefined` leaves the flag untouched.
    let nextIsSecret: boolean | undefined = undefined;
    if (localVariable.is_secret !== variable.is_secret) {
      if (localVariable.is_secret) {
        nextIsSecret = true;
      } else if (allowSecretDowngrade) {
        nextIsSecret = false;
      }
    }

    await wmill.updateVariable({
      workspace,
      path: remotePath.replaceAll(SEP, "/"),
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        ...localVariable,
        is_secret: nextIsSecret,
        ...(wsSpecific !== undefined ? { ws_specific: wsSpecific } : {}),
      },
    });
  } else {
    log.info(colors.yellow.bold(`Creating new variable ${remotePath}...`));
    await wmill.createVariable({
      workspace,
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
        ...localVariable,
        ...(wsSpecific !== undefined ? { ws_specific: wsSpecific } : {}),
      },
    });
  }
}

async function push(
  opts: GlobalOptions & { plainSecrets: boolean },
  filePath: string,
  remotePath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  log.info(colors.bold.yellow("Pushing variable..."));

  const local = parseFromFile(filePath) as VariableFile;

  // A secret value in a single-file push is authored by the user and is
  // therefore plaintext that must be encrypted server-side — unless it has the
  // shape of workspace ciphertext (a value round-tripped from `sync pull`).
  // Pushing plaintext as already-encrypted would brick the variable. An explicit
  // --plain-secrets always forces the plaintext (encrypt) path.
  let plainSecrets = opts.plainSecrets ?? false;
  if (opts.plainSecrets === undefined && local.is_secret) {
    if (!looksLikeWorkspaceCiphertext(local.value)) {
      log.info(
        colors.yellow(
          "Secret value is not in encrypted form; pushing as plaintext to be encrypted server-side (pass --plain-secrets to silence)."
        )
      );
      plainSecrets = true;
    } else {
      // The value has the shape of workspace ciphertext, so it's stored as-is.
      // A plaintext secret that coincidentally looks like ciphertext (e.g. a
      // base64 token) would be stored unreadable, so surface the assumption.
      log.warn(
        "Secret value looks already-encrypted; pushing it as-is. If it is a plaintext secret, re-run with --plain-secrets so it gets encrypted."
      );
    }
  }

  await pushVariable(
    workspace.workspaceId,
    remotePath,
    undefined,
    local,
    plainSecrets,
    undefined,
    true // single-file push is authoritative: allow secret->non-secret downgrade
  );
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

async function add(
  opts: GlobalOptions & {
    public?: boolean;
    plainSecrets?: boolean;
    yes?: boolean;
    secret?: boolean;
    description?: string;
  },
  value: string,
  remotePath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  // --secret/--no-secret take precedence over the legacy --public flag;
  // undefined means "secret on create, preserve current setting on update"
  const isSecret = opts.secret ?? (opts.public ? false : undefined);

  if (
    await wmill.existsVariable({
      workspace: workspace.workspaceId,
      path: remotePath,
    })
  ) {
    if (
      !opts.yes &&
      !(await Confirm.prompt({
        message: `Variable already exist, do you want to update its value?`,
        default: true,
      }))
    ) {
      return;
    }
    if (isSecret === false) {
      const existing = await wmill.getVariable({
        workspace: workspace.workspaceId,
        path: remotePath,
        decryptSecret: false,
      });
      if (existing.is_secret) {
        log.warn(
          colors.yellow(
            `Variable ${remotePath} is currently secret and will be downgraded to non-secret: its value will be stored in plaintext`
          )
        );
      }
    }
    log.info(colors.bold.yellow("Updating variable..."));
    await wmill.updateVariable({
      workspace: workspace.workspaceId,
      path: remotePath,
      alreadyEncrypted: false, // value from CLI is always plaintext
      requestBody: {
        value,
        ...(isSecret !== undefined ? { is_secret: isSecret } : {}),
        ...(opts.description !== undefined
          ? { description: opts.description }
          : {}),
      },
    });
  } else {
    log.info(colors.bold.yellow("Creating variable..."));
    await wmill.createVariable({
      workspace: workspace.workspaceId,
      alreadyEncrypted: false, // value from CLI is always plaintext
      requestBody: {
        path: remotePath,
        value,
        is_secret: isSecret ?? true,
        description: opts.description ?? "",
      },
    });
  }
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

const command = new Command()
  .description("variable related commands")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all variables")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a variable's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("new", "create a new variable locally")
  .arguments("<path:string>")
  .action(newVariable as any)
  .command(
    "push",
    "Push a local variable spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .option("--plain-secrets", "Push secrets as plain text")
  .action(push as any)
  .command(
    "add",
    "Create a new variable on the remote. This will update the variable if it already exists."
  )
  .arguments("<value:string> <remote_path:string>")
  .option(
    "--yes",
    "Skip confirmation prompt when updating an existing variable"
  )
  .option(
    "--secret",
    "Mark the variable as secret (default when creating a new variable)"
  )
  .option(
    "--no-secret",
    "Mark the variable as non-secret (when updating, the existing setting is preserved if neither --secret nor --no-secret is passed)"
  )
  .option(
    "--description <description:string>",
    "Set the variable description (when updating, the existing description is preserved if not passed)"
  )
  .option("--plain-secrets", "Push secrets as plain text")
  .option("--public", "Legacy option, use --no-secret instead")

  .action(add as any);

export default command;
