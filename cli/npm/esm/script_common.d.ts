export type ScriptLanguage = "python3" | "deno" | "bun" | "nativets" | "go" | "bash" | "powershell" | "postgresql" | "mysql" | "bigquery" | "snowflake" | "mssql" | "graphql" | "php";
export declare function inferContentTypeFromFilePath(contentPath: string, defaultTs: "bun" | "deno" | undefined): ScriptLanguage;
//# sourceMappingURL=script_common.d.ts.map