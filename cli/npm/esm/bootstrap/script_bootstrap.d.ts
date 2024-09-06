import { SchemaProperty } from "./common.js";
export interface ScriptMetadata {
    summary: string;
    description: string;
    lock: string | string[];
    is_template?: boolean;
    kind: string;
    schema: {
        $schema: string;
        type: string;
        properties: {
            [name: string]: SchemaProperty;
        };
        required: string[];
    };
}
export declare function defaultScriptMetadata(): ScriptMetadata;
export declare const scriptBootstrapCode: {
    python3: string;
    nativets: string;
    bun: string;
    deno: string;
    go: string;
    mysql: string;
    bigquery: string;
    snowflake: string;
    mssql: string;
    graphql: string;
    postgresql: string;
    bash: string;
    powershell: string;
    php: string;
};
//# sourceMappingURL=script_bootstrap.d.ts.map