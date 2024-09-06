import { Command, ResourceType } from "./deps.js";
export interface ResourceTypeFile {
    schema?: any;
    description?: string;
}
export declare function pushResourceType(workspace: string, remotePath: string, resource: ResourceTypeFile | ResourceType | undefined, localResource: ResourceTypeFile): Promise<void>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=resource-type.d.ts.map