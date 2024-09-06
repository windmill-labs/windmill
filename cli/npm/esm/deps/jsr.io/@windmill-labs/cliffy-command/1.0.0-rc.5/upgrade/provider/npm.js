import { Provider } from "../provider.js";
export class NpmProvider extends Provider {
    constructor({ main, logger, ...options }) {
        super({ main, logger });
        Object.defineProperty(this, "name", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "npm"
        });
        Object.defineProperty(this, "repositoryUrl", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "https://npmjs.org/"
        });
        Object.defineProperty(this, "apiUrl", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: "https://registry.npmjs.org/"
        });
        Object.defineProperty(this, "packageName", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "packageScope", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        this.packageScope = "package" in options
            ? options.package.split("/")[0].slice(1)
            : options.scope;
        this.packageName = "package" in options
            ? options.package.split("/")[1]
            : options.name;
    }
    async getVersions(name) {
        const response = await fetch(new URL(`@${this.packageScope}/${this.packageName ?? name}`, this.apiUrl));
        if (!response.ok) {
            throw new Error("couldn't fetch the latest version - try again after sometime");
        }
        const { "dist-tags": { latest }, versions } = await response
            .json();
        return {
            latest,
            versions: Object.keys(versions).reverse(),
        };
    }
    getRepositoryUrl(name, version) {
        return new URL(`package/@${this.packageScope}/${this.packageName ?? name}${version ? `/v/${version}` : ""}`, this.repositoryUrl).href;
    }
    getRegistryUrl(name, version) {
        return `npm:@${this.packageScope}/${this.packageName ?? name}@${version}`;
    }
}
