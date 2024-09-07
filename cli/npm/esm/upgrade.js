import { Provider } from "./deps.js";
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
        this.packageName = "package" in options ? options.package : options.name;
    }
    async getVersions(name) {
        const response = await fetch(new URL(`${this.packageName ?? name}`, this.apiUrl));
        if (!response.ok) {
            throw new Error("couldn't fetch the latest version - try again after sometime");
        }
        const { "dist-tags": { latest }, versions, } = (await response.json());
        return {
            latest,
            versions: Object.keys(versions).reverse(),
        };
    }
    getRepositoryUrl(name, version) {
        return new URL(`package/${this.packageName ?? name}${version ? `/v/${version}` : ""}`, this.repositoryUrl).href;
    }
    getRegistryUrl(name, version) {
        return `npm:${this.packageName ?? name}@${version}`;
    }
}
