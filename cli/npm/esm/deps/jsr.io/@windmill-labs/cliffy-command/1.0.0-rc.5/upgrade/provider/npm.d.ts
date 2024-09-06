import { Provider, type ProviderOptions, type Versions } from "../provider.js";
export type NpmProviderOptions = ProviderOptions & ({
    package: string;
} | {
    scope: string;
    name?: string;
});
export declare class NpmProvider extends Provider {
    name: string;
    private readonly repositoryUrl;
    private readonly apiUrl;
    private readonly packageName?;
    private readonly packageScope;
    constructor({ main, logger, ...options }: NpmProviderOptions);
    getVersions(name: string): Promise<Versions>;
    getRepositoryUrl(name: string, version?: string): string;
    getRegistryUrl(name: string, version: string): string;
}
//# sourceMappingURL=npm.d.ts.map