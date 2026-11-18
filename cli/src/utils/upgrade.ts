import { Provider } from "@cliffy/command/upgrade";

export type NpmProviderOptions = { main?: string; logger?: any } & (
  | {
      package: string;
    }
  | {
      scope: string;
      name?: string;
    }
);

export class NpmProvider extends Provider {
  name = "npm";
  private readonly repositoryUrl = "https://npmjs.org/";
  private readonly apiUrl = "https://registry.npmjs.org/";
  private readonly packageName?: string;

  constructor({ main, logger, ...options }: NpmProviderOptions) {
    super({ main, logger });

    this.packageName = "package" in options ? options.package : options.name;
  }

  async getVersions(name: string): Promise<any> {
    const response = await fetch(
      new URL(`${this.packageName ?? name}`, this.apiUrl)
    );
    if (!response.ok) {
      throw new Error(
        "couldn't fetch the latest version - try again after sometime"
      );
    }

    const {
      "dist-tags": { latest },
      versions,
    } = (await response.json()) as NpmApiPackageMetadata;

    return {
      latest,
      versions: Object.keys(versions).reverse(),
    };
  }

  getRepositoryUrl(name: string, version?: string): string {
    return new URL(
      `package/${this.packageName ?? name}${version ? `/v/${version}` : ""}`,
      this.repositoryUrl
    ).href;
  }

  getRegistryUrl(name: string, version: string): string {
    return `npm:${this.packageName ?? name}@${version}`;
  }

  async hasRequiredPermissions(): Promise<boolean> {
    return true;
  }

  // Cliffy's default runtime runs `npm install --silent`, so a failed upgrade
  // throws with an empty message. Override to run npm without --silent and put
  // the captured output + recovery hint in the error, so failures are actionable.
  override async upgrade({
    name,
    to,
    main,
    verbose,
  }: {
    name: string;
    to: string;
    main?: string;
    verbose?: boolean;
  }): Promise<void> {
    const specifier = this.getSpecifier(name, to, main).replace(/^npm:/, "");
    const args = ["install", "--global", "--force", specifier];
    this.logger?.log(`$ npm ${args.join(" ")}`);
    const { spawn } = await import("node:child_process");
    const HINT =
      "If this is a permissions error, retry with sudo, or reinstall with: npm uninstall -g windmill-cli && npm install -g windmill-cli";
    // `shell: true` so `npm` resolves on Windows (npm.cmd) — same spawn
    // pattern as commands/app/bundle.ts.
    const proc = spawn("npm", args, {
      stdio: [null, "pipe", "pipe"],
      shell: true,
    });
    const output: string[] = [];
    proc.stdout?.on("data", (d) => {
      output.push(String(d));
      if (verbose) process.stdout.write(d);
    });
    proc.stderr?.on("data", (d) => {
      output.push(String(d));
      if (verbose) process.stderr.write(d);
    });
    // `error` fires when npm can't be spawned at all; without a listener that
    // is an uncaught exception, crashing before any hint prints. `close`
    // resolves null when npm died from a signal — a failure too.
    const exitCode: number | null = await new Promise<number | null>(
      (resolve, reject) => {
        proc.on("error", reject);
        proc.on("close", resolve);
      }
    ).catch((e) => {
      throw new Error(
        `failed to run npm: ${e instanceof Error ? e.message : e}\n\n${HINT}`
      );
    });
    if (exitCode !== 0) {
      const detail = output.join("").trim();
      throw new Error(
        `npm exited with ${
          exitCode === null ? "a signal" : `code ${exitCode}`
        }${detail ? `:\n${detail}` : ""}\n\n${HINT}`
      );
    }
  }
}

type NpmApiPackageMetadata = {
  "dist-tags": {
    latest: string;
  };
  versions: {
    [version: string]: unknown;
  };
};
