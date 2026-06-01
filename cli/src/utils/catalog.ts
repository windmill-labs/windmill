import { colors } from "@cliffy/ansi/colors";
import { Table } from "@cliffy/table";

import * as wmill from "../../gen/services.gen.ts";
import { Preview } from "../../gen/types.gen.ts";
import { requireLogin } from "../core/auth.ts";
import { resolveWorkspace } from "../core/context.ts";
import * as log from "../core/log.ts";
import { GlobalOptions } from "../types.ts";
import { pollJobWithQueueLogging } from "./job_polling.ts";

// Shared building blocks for SQL catalogs (`wmill datatable` and
// `wmill ducklake`). Both expose a SQL surface backed by Windmill script
// previews, but the language and arg shape differ — `buildCatalogQueryPlan`
// centralizes that difference so subcommands stay thin.

export type CatalogKind = "datatable" | "ducklake";

interface CatalogQueryPlan {
  language: NonNullable<Preview["language"]>;
  content: string;
  args: Record<string, unknown>;
}

function buildCatalogQueryPlan(
  kind: CatalogKind,
  name: string,
  sql: string,
): CatalogQueryPlan {
  switch (kind) {
    case "datatable":
      return {
        language: "postgresql",
        content: sql,
        args: { database: `datatable://${name}` },
      };
    case "ducklake": {
      const attach = `ATTACH 'ducklake://${name}' AS dl;\nUSE dl;\n`;
      return {
        language: "duckdb",
        content: attach + sql,
        args: {},
      };
    }
  }
}

export async function runCatalogQuery(
  opts: GlobalOptions & { silent?: boolean },
  kind: CatalogKind,
  name: string,
  sql: string,
): Promise<void> {
  if (opts.silent) {
    log.setSilent(true);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const plan = buildCatalogQueryPlan(kind, name, sql);

  log.info(colors.gray(`Running query on ${kind}://${name}`));

  const jobId = await wmill.runScriptPreview({
    workspace: workspace.workspaceId,
    requestBody: {
      content: plan.content,
      language: plan.language,
      args: plan.args,
    },
  });

  const { result, success } = await pollJobWithQueueLogging(
    workspace.workspaceId,
    jobId,
  );

  if (!success) {
    process.exitCode = 1;
    if (opts.silent) {
      console.log(JSON.stringify(result));
    } else {
      log.info(colors.red.bold("Query failed"));
      log.info(JSON.stringify(result, null, 2));
    }
    return;
  }

  if (opts.silent) {
    console.log(JSON.stringify(result));
  } else {
    renderQueryResult(result);
  }
}

/**
 * Render a SQL query result. Postgres script previews return rows as an array
 * of `{column: value}` objects — display those as a table. Anything else
 * (DDL output, empty results, scalar payloads) falls back to pretty JSON.
 */
function renderQueryResult(result: unknown): void {
  if (Array.isArray(result) && result.length > 0 && result.every(isRecord)) {
    const rows = result as Record<string, unknown>[];
    const columns = collectColumns(rows);
    new Table()
      .header(columns)
      .padding(2)
      .border(true)
      .body(rows.map((row) => columns.map((c) => formatCell(row[c]))))
      .render();
    return;
  }
  log.info(JSON.stringify(result, null, 2));
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return v !== null && typeof v === "object" && !Array.isArray(v);
}

function collectColumns(rows: Record<string, unknown>[]): string[] {
  const seen = new Set<string>();
  const columns: string[] = [];
  for (const row of rows) {
    for (const key of Object.keys(row)) {
      if (!seen.has(key)) {
        seen.add(key);
        columns.push(key);
      }
    }
  }
  return columns;
}

function formatCell(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return JSON.stringify(value);
}
