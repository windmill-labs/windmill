import { Command } from "./deps.js";
interface SimplifiedUser {
    role: string;
    username: string;
    disabled: boolean;
}
export declare function pushWorkspaceUser(workspace: string, path: string, user: SimplifiedUser | undefined, localUser: SimplifiedUser): Promise<void>;
interface SimplifiedGroup {
    summary: string | undefined;
    admins: string[];
    members: string[];
}
export declare function pushGroup(workspace: string, path: string, group: SimplifiedGroup | undefined, localGroup: SimplifiedGroup): Promise<void>;
export declare function pullInstanceUsers(preview?: boolean): Promise<number | undefined>;
export declare function pushInstanceUsers(preview?: boolean): Promise<number | undefined>;
export declare function pullInstanceGroups(preview?: boolean): Promise<number | undefined>;
export declare function pushInstanceGroups(preview?: boolean): Promise<number | undefined>;
declare const command: Command<void, void, {
    email?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType | undefined;
} & {
    password?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType | undefined;
}, [], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=user.d.ts.map