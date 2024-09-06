export interface DifferenceCreate {
    type: "CREATE";
    path: (string | number)[];
    value: any;
}
export interface DifferenceRemove {
    type: "REMOVE";
    path: (string | number)[];
    oldValue: any;
}
export interface DifferenceChange {
    type: "CHANGE";
    path: (string | number)[];
    value: any;
    oldValue: any;
}
export type Difference = DifferenceCreate | DifferenceRemove | DifferenceChange;
export type GlobalOptions = {
    workspace: string | undefined;
    token: string | undefined;
};
export declare function isSuperset(subset: Record<string, any>, superset: Record<string, any>): boolean;
export declare function showDiff(local: string, remote: string): void;
export declare function showConflict(path: string, local: string, remote: string): void;
export declare function pushObj(workspace: string, p: string, befObj: any, newObj: any, plainSecrets: boolean, message?: string): Promise<void>;
export declare function parseFromPath(p: string, content: string): any;
export declare function parseFromFile(p: string): any;
export declare function getTypeStrFromPath(p: string): "script" | "variable" | "flow" | "resource" | "resource-type" | "folder" | "app" | "schedule" | "user" | "group" | "settings" | "encryption_key";
export declare function removeType(str: string, type: string): string;
export declare function removePathPrefix(str: string, prefix: string): string;
//# sourceMappingURL=types.d.ts.map