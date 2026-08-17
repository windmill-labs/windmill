import { Command } from "@cliffy/command";
import * as fs from "node:fs";
import * as path from "node:path";
import * as log from "../../core/log.ts";
import { colors } from "@cliffy/ansi/colors";
import * as windmillUtils from "@windmill-labs/shared-utils";

interface GeneratePolicyOptions {
  out?: string;
}

/**
 * Derive a raw app's execution policy from its runnables, without deploying it.
 * `wmill app push` derives the same policy as part of deploying; this exposes it
 * on its own so anything that needs a raw app's policy — the server-side deploy
 * behind `/apps/create_raw_source`, most of all — runs this exact derivation.
 *
 * It has to be exact: `triggerables_v2` is the allowlist the server matches every
 * run against, keyed by `<component>:rawscript/<sha256 of the inline code>` (or
 * `<component>:<script|flow>/<path>`). A key derived any other way leaves the
 * deployed app's runnables "forbidden by policy".
 *
 * The result goes to a file, not stdout, for the same reason `app bundle` does:
 * the command logs as it runs, so a caller can't read the result off the pipe.
 */
async function generatePolicy(
  opts: GeneratePolicyOptions,
  runnablesFile: string,
) {
  const raw = fs.readFileSync(path.resolve(runnablesFile), "utf-8");
  let runnables: Record<string, unknown>;
  try {
    runnables = JSON.parse(raw);
  } catch (e) {
    throw new Error(`${runnablesFile} is not valid JSON: ${e}`);
  }
  if (typeof runnables !== "object" || runnables === null) {
    throw new Error(`${runnablesFile} must hold the app's runnables object`);
  }

  const policy = await windmillUtils.updateRawAppPolicy(
    runnables as any,
    undefined,
  );

  const out = path.resolve(opts.out ?? "policy.json");
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, JSON.stringify(policy));
  log.info(colors.green(`Wrote policy to ${out}`));
}

const command = new Command()
  .description(
    "Derive a raw app's policy from its runnables without deploying it",
  )
  .arguments("<runnables_file:string>")
  .option(
    "--out <file:string>",
    "File to write the policy JSON into (default: policy.json)",
  )
  .action(generatePolicy as any);

export default command;
