import { bold, brightBlue } from "../../../../@std/fmt/0.225.6/colors.js";
import { ValidationError } from "../_errors.js";
import { exit } from "../../../cliffy-internal/1.0.0-rc.5/runtime/exit.js";
import { Command } from "../command.js";
import { EnumType } from "../types/enum.js";
import { createLogger } from "./logger.js";
import type { Provider, Versions } from "./provider.js";
import { Spinner } from "./spinner.js";
import {
  type RuntimeOptions,
  type RuntimeOptionsMap,
  upgrade,
} from "./upgrade.js";

export interface UpgradeCommandOptions<
  TProvider extends Provider = Provider,
> extends RuntimeOptions {
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
export class UpgradeCommand extends Command {
  private readonly providers: ReadonlyArray<Provider>;

  constructor(
    { provider, ...options }: UpgradeCommandOptions,
  ) {
    super();
    this.providers = Array.isArray(provider) ? provider : [provider];

    if (!this.providers.length) {
      throw new Error(`No upgrade provider defined!`);
    }

    this
      .description(() =>
        `Upgrade ${this.getMainCommand().getName()} executable to latest or given version.`
      )
      .noGlobals()
      .type("provider", new EnumType(this.getProviderNames()))
      .option(
        "-r, --registry <name:provider>",
        `The registry name from which to upgrade.`,
        {
          default: this.getProvider().name,
          hidden: this.providers.length < 2,
          value: (registry) => this.getProvider(registry),
        },
      )
      .option(
        "-l, --list-versions",
        "Show available versions.",
        {
          action: async ({ registry }) => {
            await registry.listVersions(
              this.getMainCommand().getName(),
              this.getVersion(),
            );
            exit(0);
          },
        },
      )
      .option(
        "--version <version:string:version>",
        "The version to upgrade to.",
        { default: "latest" },
      )
      .option(
        "-f, --force",
        "Replace current installation even if not out-of-date.",
      )
      .option(
        "-v, --verbose",
        "Log verbose output.",
      )
      .option("--no-spinner", "Disable spinner.")
      .complete("version", () => this.getAllVersions())
      .action(
        async (
          {
            registry: provider,
            version,
            force,
            verbose,
            spinner: spinnerEnabled,
          },
        ) => {
          const name: string = this.getMainCommand().getName();
          const currentVersion: string | undefined = this.getVersion();

          const spinner = spinnerEnabled
            ? new Spinner({
              message: brightBlue(
                `Upgrading ${bold(name)} from version ${
                  bold(currentVersion ?? "")
                } to ${bold(version)}...`,
              ),
            })
            : undefined;
          const logger = createLogger({ spinner, verbose });
          spinner?.start();
          provider.setLogger(logger);

          try {
            await upgrade({
              name,
              to: version,
              from: currentVersion,
              force,
              provider,
              verbose,
              logger,
              ...options,
            });
          } catch (error: unknown) {
            logger.error(
              !verbose && error instanceof Error ? error.message : error,
            );
            spinner?.stop();
            exit(1);
          } finally {
            spinner?.stop();
          }
        },
      );
  }

  public async getAllVersions(): Promise<Array<string>> {
    const { versions } = await this.getVersions();
    return versions;
  }

  public async getLatestVersion(): Promise<string> {
    const { latest } = await this.getVersions();
    return latest;
  }

  public getVersions(): Promise<Versions> {
    return this.getProvider().getVersions(
      this.getMainCommand().getName(),
    );
  }

  private getProvider(name?: string): Provider {
    const provider = name
      ? this.providers.find((provider) => provider.name === name)
      : this.providers[0];
    if (!provider) {
      throw new ValidationError(`Unknown provider "${name}"`);
    }
    return provider;
  }

  private getProviderNames(): Array<string> {
    return this.providers.map((provider) => provider.name);
  }
}
