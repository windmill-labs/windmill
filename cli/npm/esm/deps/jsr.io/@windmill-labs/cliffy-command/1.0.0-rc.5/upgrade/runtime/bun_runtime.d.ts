import type { Logger } from "../logger.js";
import { NodeRuntime } from "./node_runtime.js";
export declare class BunRuntime extends NodeRuntime {
    protected execute(cmdArgs: string[], isJsr: boolean, logger?: Logger): Promise<void>;
}
//# sourceMappingURL=bun_runtime.d.ts.map