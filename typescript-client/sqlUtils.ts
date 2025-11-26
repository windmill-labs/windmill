import { getWorkspace, JobService } from "./client";

export type SqlStatement = {
  queryStr: string;
  args: Record<string, any>;
  query(): Promise<any>;
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
 *     WHERE name = ${name}
 *     LIMIT ${age}::int
 * `.query()
 */
export function datatable(name: string = "main"): SqlTemplateFunction {
  let sql: SqlTemplateFunction = (
    strings: TemplateStringsArray,
    ...values: any[]
  ) => {
    let queryStr =
      values.map((_, i) => `-- $${i + 1} arg${i + 1}`).join("\n") + "\n";
    for (let i = 0; i < strings.length; i++) {
      queryStr += strings[i];
      if (i !== strings.length - 1) queryStr += `$${i + 1}`;
    }
    const args = {
      ...Object.fromEntries(values.map((v, i) => [`arg${i + 1}`, v])),
      database: `datatable://${name}`,
    };

    return {
      queryStr,
      args,
      query: async () => {
        let result = await JobService.runScriptPreviewInline({
          workspace: getWorkspace(),
          requestBody: {
            args,
            content: queryStr,
            language: "postgresql",
          },
        });
        return result;
      },
    } satisfies SqlStatement;
  };
  return sql;
}
