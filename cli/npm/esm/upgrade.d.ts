import { Provider } from "./deps.js";
export type NpmProviderOptions = {
    main?: string;
    logger?: any;
} & ({
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
    constructor({ main, logger, ...options }: NpmProviderOptions);
    getVersions(name: string): Promise<any>;
    getRepositoryUrl(name: string, version?: string): string;
    getRegistryUrl(name: string, version: string): string;
}
//# sourceMappingURL=upgrade.d.ts.map