import type { Logger } from "./logger.js";
export interface Versions {
    latest: string;
    versions: Array<string>;
}
/** Shared provider options. */
export interface ProviderOptions {
    main?: string;
    logger?: Logger;
}
/** Provider upgrade options. */
export interface ProviderUpgradeOptions {
    name: string;
    to: string;
    main?: string;
    args?: Array<string>;
    from?: string;
    force?: boolean;
    verbose?: boolean;
}
/**
 * Upgrade provider.
 *
 * The upgrade provider is an api wrapper for a javascript registry which is
 * used by the upgrade command to upgrade the cli to a specific version.
 *
 * @example Upgrade provider example.
 *
 * ```
 * import { Command } from "@cliffy/command";
 * import { UpgradeCommand } from "@cliffy/command/upgrade";
 * import { DenoLandProvider } from "@cliffy/command/upgrade/provider/deno-land";
 * import { GithubProvider } from "@cliffy/command/upgrade/provider/github";
 * import { JsrProvider } from "@cliffy/command/upgrade/provider/jsr";
 * import { NestLandProvider } from "@cliffy/command/upgrade/provider/nest-land";
 * import { NpmProvider } from "@cliffy/command/upgrade/provider/npm";
 *
 * const upgradeCommand = new UpgradeCommand({
 *   provider: [
 *     new JsrProvider({ package: "@examples/package" }),
 *   ],
 * });
 * ```
 */
export declare abstract class Provider {
    abstract readonly name: string;
    protected readonly main?: string;
    protected readonly maxListSize: number;
    protected logger: Logger;
    private maxCols;
    protected constructor({ main, logger }?: ProviderOptions);
    abstract getVersions(name: string): Promise<Versions>;
    abstract getRepositoryUrl(name: string, version?: string): string;
    abstract getRegistryUrl(name: string, version: string): string;
    upgrade?(options: ProviderUpgradeOptions): Promise<void>;
    getSpecifier(name: string, version: string, defaultMain?: string): string;
    isOutdated(name: string, currentVersion: string, targetVersion: string): Promise<boolean>;
    listVersions(name: string, currentVersion?: string): Promise<void>;
    protected printVersions(versions: Array<string>, currentVersion?: string, { maxCols, indent }?: {
        maxCols?: number;
        indent?: number;
    }): void;
    setLogger(logger: Logger): void;
    private getMain;
}
//# sourceMappingURL=provider.d.ts.map