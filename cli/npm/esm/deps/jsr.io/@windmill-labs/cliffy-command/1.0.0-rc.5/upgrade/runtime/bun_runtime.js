import * as dntShim from "../../../../../../../_dnt.shims.js";
import { dim } from "../../../../../@std/fmt/0.225.6/colors.js";
import { NodeRuntime } from "./node_runtime.js";
export class BunRuntime extends NodeRuntime {
    async execute(cmdArgs, isJsr, logger) {
        // deno-lint-ignore no-explicit-any
        const Bun = dntShim.dntGlobalThis.Bun;
        // deno-lint-ignore no-explicit-any
        const process = dntShim.dntGlobalThis.process;
        cmdArgs = isJsr
            ? [`${process.execPath}x`, "jsr", ...cmdArgs]
            : [process.execPath, ...cmdArgs];
        logger?.log(dim("$ %s"), cmdArgs.join(" "));
        const proc = Bun.spawn(cmdArgs, { stdout: "pipe", stderr: "pipe" });
        await proc.exited;
        if (proc.exitCode) {
            const stderr = await new Response(proc.stderr).text();
            throw new Error(stderr.trim());
        }
    }
}
