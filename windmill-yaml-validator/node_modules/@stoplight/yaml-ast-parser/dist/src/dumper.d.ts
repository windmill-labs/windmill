import { SchemaDefinition } from "./schema";
export interface DumpOptions {
    indent?: number;
    noArrayIndent?: boolean;
    skipInvalid?: boolean;
    flowLevel?: number;
    styles?: {
        [x: string]: any;
    };
    schema?: SchemaDefinition;
    lineWidth?: number;
    noRefs?: boolean;
    comments?: {
        [x: string]: Comment[];
    };
}
export declare type Comment = Readonly<{
    value: string;
    placement: 'before-eol' | 'leading' | 'trailing' | 'between';
} & ({} | {
    placement: 'between';
    between: [string, string];
})>;
export declare function dump(input: any, options?: DumpOptions): string;
export declare function safeDump(input: any, options?: DumpOptions): string;
