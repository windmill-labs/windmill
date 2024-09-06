import type { RuntimeUpgradeOptions } from "./runtime.js";
import type { DenoRuntimeOptions } from "./runtime/deno_runtime.js";
/** Shared runtime options. */
export interface RuntimeOptions {
    args?: Array<string>;
    main?: string;
}
/** Runtime options map for supported runtimes. */
export interface RuntimeOptionsMap {
    deno?: RuntimeOptions & DenoRuntimeOptions;
    node?: RuntimeOptions;
    bun?: RuntimeOptions;
}
/**
 * Options for upgrading a package from a provided registry with any supported
 * runtimes.
 * Currently supported runtimes are: `deno`, `node` and `bun`.
 */
export interface UpgradeOptions extends RuntimeUpgradeOptions {
    runtime?: RuntimeOptionsMap;
}
/**
 * Upgrade a package from given registry.
 * Runtime is auto-detected. Currently supported runtimes are: `deno`, `node` and `bun`.
 */
export declare function upgrade({ runtime: runtimeOptions, provider, ...options }: UpgradeOptions): Promise<void>;
//# sourceMappingURL=upgrade.d.ts.map