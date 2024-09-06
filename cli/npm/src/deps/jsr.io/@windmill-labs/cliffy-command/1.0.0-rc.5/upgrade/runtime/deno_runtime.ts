import * as dntShim from "../../../../../../../_dnt.shims.js";
import { dim } from "../../../../../@std/fmt/0.225.6/colors.js";
import { Runtime, type RuntimeUpgradeOptions } from "../runtime.js";
import type { Logger } from "../logger.js";

/** Deno specific upgrade options. */
export interface DenoRuntimeOptions {
  importMap?: string;
}

/** Deno specific package upgrade options. */
export type DenoRuntimeUpgradeOptions =
  & RuntimeUpgradeOptions
  & DenoRuntimeOptions;

/** Deno runtime upgrade handler. */
export class DenoRuntime extends Runtime {
  async upgrade(
    { provider, name, main, to, importMap, verbose, logger, args = [] }:
      DenoRuntimeUpgradeOptions,
  ): Promise<void> {
    const specifier: string = provider.getSpecifier(name, to, main);

    const cmdArgs = ["install", `--name=${name}`, "--global", "--force"];

    if (!verbose) {
      cmdArgs.push("--quiet");
    }

    if (args.length) {
      cmdArgs.push(...args);
    }

    if (importMap) {
      const importJson: string = new URL(importMap, specifier).href;
      cmdArgs.push(`--import-map=${importJson}`);
    }

    cmdArgs.push(specifier);

    await this.execute(cmdArgs, logger);
  }

  protected async execute(
    cmdArgs: string[],
    logger?: Logger,
  ): Promise<void> {
    logger?.log(
      dim("$ %s %s"),
      dntShim.Deno.execPath(),
      cmdArgs.join(" "),
    );

    const cmd = new dntShim.Deno.Command(dntShim.Deno.execPath(), {
      args: cmdArgs,
      stdout: "piped",
      stderr: "piped",
    });
    const { success, stderr } = await cmd.output();

    if (!success) {
      throw new Error(new TextDecoder().decode(stderr).trim());
    }
  }
}
