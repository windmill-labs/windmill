import { getWorkspace, JobService } from "./client";

export type SqlStatement = {
  content: string;
  args: Record<string, any>;
  fetch(): Promise<any>;
};

export interface SqlTemplateFunction {
  (strings: TemplateStringsArray, ...values: any[]): SqlStatement;
}

/**
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
export function datatable(name: string = "main"): SqlTemplateFunction {
  return sqlProviderImpl(name, "datatable");
}

/**
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 */
export function ducklake(name: string = "main"): SqlTemplateFunction {
  return sqlProviderImpl(name, "ducklake");
}

function sqlProviderImpl(
  name: string,
  provider: "datatable" | "ducklake"
): SqlTemplateFunction {
  let sql: SqlTemplateFunction = (
    strings: TemplateStringsArray,
    ...values: any[]
  ) => {
    let formatArg = {
      datatable: (i: number) => `-- $${i + 1} arg${i + 1}`,
      ducklake: (i: number) => `-- $arg${i + 1}`,
    }[provider];
    let content = values.map((_, i) => formatArg(i)).join("\n") + "\n";
    if (provider === "ducklake")
      content += `ATTACH 'ducklake://${name}' AS dl;USE dl;\n`;
    for (let i = 0; i < strings.length; i++) {
      content += strings[i];
      if (i !== strings.length - 1) content += `$${i + 1}`;
    }

    const args = {
      ...Object.fromEntries(values.map((v, i) => [`arg${i + 1}`, v])),
      ...(provider === "datatable" ? { database: `datatable://${name}` } : {}),
    };
    const language = {
      datatable: "postgresql" as const,
      ducklake: "duckdb" as const,
    }[provider];

    return {
      content,
      args,
      fetch: async () => {
        let result = await JobService.runScriptPreviewInline({
          workspace: getWorkspace(),
          requestBody: { args, content, language },
        });
        return result;
      },
    } satisfies SqlStatement;
  };
  return sql;
}
