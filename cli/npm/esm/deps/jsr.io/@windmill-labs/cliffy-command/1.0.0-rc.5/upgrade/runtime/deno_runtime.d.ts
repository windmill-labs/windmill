import { Runtime, type RuntimeUpgradeOptions } from "../runtime.js";
import type { Logger } from "../logger.js";
/** Deno specific upgrade options. */
export interface DenoRuntimeOptions {
    importMap?: string;
}
/** Deno specific package upgrade options. */
export type DenoRuntimeUpgradeOptions = RuntimeUpgradeOptions & DenoRuntimeOptions;
/** Deno runtime upgrade handler. */
export declare class DenoRuntime extends Runtime {
    upgrade({ provider, name, main, to, importMap, verbose, logger, args }: DenoRuntimeUpgradeOptions): Promise<void>;
    protected execute(cmdArgs: string[], logger?: Logger): Promise<void>;
}
//# sourceMappingURL=deno_runtime.d.ts.map