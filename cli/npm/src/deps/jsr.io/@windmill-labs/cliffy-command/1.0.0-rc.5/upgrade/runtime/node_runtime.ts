import { dim } from "../../../../../@std/fmt/0.225.6/colors.js";
import type { Logger } from "../logger.js";
import type { Runtime, RuntimeUpgradeOptions } from "../runtime.js";

export class NodeRuntime implements Runtime {
  upgrade(
    {
      provider,
      name,
      main,
      to,
      verbose,
      logger,
      args = [],
    }: RuntimeUpgradeOptions,
  ): Promise<void> {
    const specifier = provider.getSpecifier(name, to, main)
      .replace(/^(npm|jsr):/, "");
    const isJsr = provider.name === "jsr";

    const cmdArgs = ["install", "--global", "--force"];

    if (!verbose) {
      cmdArgs.push("--silent");
    }

    if (args.length) {
      cmdArgs.push(...args);
    }

    cmdArgs.push(specifier);

    return this.execute(cmdArgs, isJsr, logger);
  }

  protected async execute(
    cmdArgs: string[],
    isJsr: boolean,
    logger?: Logger,
  ): Promise<void> {
    const { spawn } = await import("node:child_process");

    const [bin, args] = isJsr ? ["npx", ["jsr", ...cmdArgs]] : ["npm", cmdArgs];

    logger?.log(
      dim("$ %s %s %s"),
      bin,
      args.join(" "),
    );

    const proc = spawn(bin, args, { stdio: [null, "pipe", "pipe"] });
    const stderr: Array<string> = [];

    proc.stderr?.on("data", (data: string) => stderr.push(data.toString()));

    const exitCode: number = await new Promise(
      (resolve) => proc.on("close", resolve),
    );

    if (exitCode) {
      throw new Error(stderr.join("\n").trim());
    }
  }
}
