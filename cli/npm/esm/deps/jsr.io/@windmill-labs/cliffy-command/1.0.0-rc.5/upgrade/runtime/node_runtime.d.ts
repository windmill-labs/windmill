import type { Logger } from "../logger.js";
import type { Runtime, RuntimeUpgradeOptions } from "../runtime.js";
export declare class NodeRuntime implements Runtime {
    upgrade({ provider, name, main, to, verbose, logger, args, }: RuntimeUpgradeOptions): Promise<void>;
    protected execute(cmdArgs: string[], isJsr: boolean, logger?: Logger): Promise<void>;
}
//# sourceMappingURL=node_runtime.d.ts.map