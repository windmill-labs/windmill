import { Command } from "../command.js";
import type { Provider, Versions } from "./provider.js";
import { type RuntimeOptions, type RuntimeOptionsMap } from "./upgrade.js";
export interface UpgradeCommandOptions<TProvider extends Provider = Provider> extends RuntimeOptions {
    provider: TProvider | Array<TProvider>;
    runtime?: RuntimeOptionsMap;
}
/**
 * The `UpgradeCommand` adds an upgrade functionality to the cli to be able to
 * seamlessly upgrade the cli to the latest or a specific version from a
 * provided registry with any supported runtime.
 * Currently supported runtimes are: `deno`, `node` and `bun`.
 *
 * @example Upgrade command example.
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
 * await new Command()
 *   .name("my-cli")
 *   .version("0.2.1")
 *   .command(
 *     "upgrade",
 *     new UpgradeCommand({
 *       provider: [
 *         new JsrProvider({ scope: "examples" }),
 *         new NpmProvider({ scope: "examples" }),
 *         new DenoLandProvider(),
 *         new NestLandProvider(),
 *         new GithubProvider({ repository: "examples/my-cli" }),
 *       ],
 *     }),
 *   )
 *   .parse();
 * ```
 */
export declare class UpgradeCommand extends Command {
    private readonly providers;
    constructor({ provider, ...options }: UpgradeCommandOptions);
    getAllVersions(): Promise<Array<string>>;
    getLatestVersion(): Promise<string>;
    getVersions(): Promise<Versions>;
    private getProvider;
    private getProviderNames;
}
//# sourceMappingURL=upgrade_command.d.ts.map