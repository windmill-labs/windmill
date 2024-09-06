import { dim } from "../../../../../@std/fmt/0.225.6/colors.js";
export class NodeRuntime {
    upgrade({ provider, name, main, to, verbose, logger, args = [], }) {
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
    async execute(cmdArgs, isJsr, logger) {
        const { spawn } = await import("node:child_process");
        const [bin, args] = isJsr ? ["npx", ["jsr", ...cmdArgs]] : ["npm", cmdArgs];
        logger?.log(dim("$ %s %s %s"), bin, args.join(" "));
        const proc = spawn(bin, args, { stdio: [null, "pipe", "pipe"] });
        const stderr = [];
        proc.stderr?.on("data", (data) => stderr.push(data.toString()));
        const exitCode = await new Promise((resolve) => proc.on("close", resolve));
        if (exitCode) {
            throw new Error(stderr.join("\n").trim());
        }
    }
}
