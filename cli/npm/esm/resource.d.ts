import { Command, Resource } from "./deps.js";
export interface ResourceFile {
    value: any;
    description?: string;
    resource_type: string;
    is_oauth?: boolean;
}
export declare function pushResource(workspace: string, remotePath: string, resource: ResourceFile | Resource | undefined, localResource: ResourceFile): Promise<void>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=resource.d.ts.map