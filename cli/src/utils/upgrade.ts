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
}

type NpmApiPackageMetadata = {
  "dist-tags": {
    latest: string;
  };
  versions: {
    [version: string]: unknown;
  };
};
