import { Provider, type ProviderOptions, type Versions } from "../provider.js";

export type NpmProviderOptions =
  & ProviderOptions
  & ({
    package: string;
  } | {
    scope: string;
    name?: string;
  });

export class NpmProvider extends Provider {
  name = "npm";
  private readonly repositoryUrl = "https://npmjs.org/";
  private readonly apiUrl = "https://registry.npmjs.org/";
  private readonly packageName?: string;
  private readonly packageScope: string;

  constructor({ main, logger, ...options }: NpmProviderOptions) {
    super({ main, logger });
    this.packageScope = "package" in options
      ? options.package.split("/")[0].slice(1)
      : options.scope;
    this.packageName = "package" in options
      ? options.package.split("/")[1]
      : options.name;
  }

  async getVersions(
    name: string,
  ): Promise<Versions> {
    const response = await fetch(
      new URL(`@${this.packageScope}/${this.packageName ?? name}`, this.apiUrl),
    );
    if (!response.ok) {
      throw new Error(
        "couldn't fetch the latest version - try again after sometime",
      );
    }

    const { "dist-tags": { latest }, versions } = await response
      .json() as NpmApiPackageMetadata;

    return {
      latest,
      versions: Object.keys(versions).reverse(),
    };
  }

  getRepositoryUrl(name: string, version?: string): string {
    return new URL(
      `package/@${this.packageScope}/${this.packageName ?? name}${
        version ? `/v/${version}` : ""
      }`,
      this.repositoryUrl,
    ).href;
  }

  getRegistryUrl(name: string, version: string): string {
    return `npm:@${this.packageScope}/${this.packageName ?? name}@${version}`;
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
