import { Command } from "./deps.js";
export interface Instance {
    remote: string;
    name: string;
    token: string;
    prefix: string;
}
export declare function allInstances(): Promise<Instance[]>;
export declare function addInstance(): Promise<{
    name: string;
    remote: string;
    token: string;
    prefix: string;
}>;
type CompareObject<T extends string> = {
    [K in T]: string;
};
export declare function compareInstanceObjects<T extends string>(fromObjects: CompareObject<T>[], toObjects: CompareObject<T>[], idProp: T, objectName: string): number;
declare const command: Command<void, void, {
    skipUsers?: true | undefined;
} & {
    skipSettings?: true | undefined;
} & {
    skipConfigs?: true | undefined;
} & {
    skipGroups?: true | undefined;
} & {
    includeWorkspaces?: true | undefined;
} & {
    baseUrl?: true | undefined;
}, [], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=instance.d.ts.map