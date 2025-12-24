import { colors, Command, log, yamlParseFile } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { DataTableSchema } from "../../../gen/types.gen.ts";
import path from "node:path";
import * as fs from "node:fs";

interface GenerateDatatablesOptions extends GlobalOptions {
  output?: string;
}

/**
 * Generates DATATABLES.md content from remote workspace datatable schemas
 */
function generateDatatablesMarkdown(
  schemas: DataTableSchema[],
  localData?: {
    tables?: string[];
    datatable?: string;
    schema?: string;
  }
): string {
  const defaultDatatable = localData?.datatable;
  const defaultSchema = localData?.schema;
  const tables = localData?.tables ?? [];

  let content = `# Datatable Configuration

This file documents the available datatables and their schemas in the connected Windmill workspace.

## ⚠️ CRITICAL RULES FOR AI AGENTS

**READ THIS FIRST - These rules are mandatory:**

1. **ONLY USE WHITELISTED TABLES**: You can ONLY query tables listed in \`raw_app.yaml\` → \`data.tables\`.
   Tables not in this list are NOT accessible to the app, even if they exist in the database.

2. **ADD TABLES BEFORE USING THEM**: If you need a table not in \`data.tables\`, you MUST:
   - First add it to \`data.tables\` in \`raw_app.yaml\`
   - Then use it in your queries

3. **PRIORITIZE CONFIGURED DATATABLE/SCHEMA**: When looking for tables:
   - First, check the whitelisted tables below
   - If creating new tables, use the default datatable${defaultSchema ? ` and schema (\`${defaultSchema}\`)` : ''} shown below
   - The schema reference below helps you discover what tables exist, but you must whitelist them first

## App Configuration

${defaultDatatable
    ? `**Default Datatable:** \`${defaultDatatable}\`${defaultSchema ? ` | **Default Schema:** \`${defaultSchema}\`` : ''}

→ When creating new tables, use this datatable${defaultSchema ? ` and schema` : ''}.`
    : `**No default datatable configured.** Set \`data.datatable\` in \`raw_app.yaml\` before creating tables.`}

### Whitelisted Tables (USE THESE FIRST)

${tables.length > 0
    ? `The app has access to these tables - **use these in your queries**:\n\n${tables.map((t) => `- \`${t}\``).join("\n")}\n\n→ To use any other table from the schemas below, add it to \`data.tables\` in \`raw_app.yaml\` first.`
    : `**No tables are currently whitelisted.**\n\nTo use any table from the schemas below, add it to \`data.tables\` in \`raw_app.yaml\` first:\n\n\`\`\`yaml\ndata:\n  datatable: ${defaultDatatable || 'main'}\n  tables:\n    - ${defaultDatatable || 'main'}/${defaultSchema ? defaultSchema + ':' : ''}table_name\n\`\`\``}

`;

  // Add schemas section
  content += `## Available Datatables and Schemas (Reference Only)

> **Note**: These schemas show what tables exist in the workspace. To use any table,
> you must first add it to \`data.tables\` in \`raw_app.yaml\`.

`;

  if (schemas.length === 0) {
    content += `*No datatables are configured in this workspace.*

Configure datatables in Workspace Settings > Windmill Data Tables.
`;
  } else {
    for (const dt of schemas) {
      content += `### Datatable: \`${dt.datatable_name}\`

`;

      if (dt.error) {
        content += `> ⚠️ Error loading schema: ${dt.error}

`;
        continue;
      }

      if (!dt.schemas || Object.keys(dt.schemas).length === 0) {
        content += `*No schemas found in this datatable.*

`;
        continue;
      }

      for (const [schemaName, tables] of Object.entries(dt.schemas)) {
        content += `#### Schema: \`${schemaName}\`

`;

        if (!tables || Object.keys(tables).length === 0) {
          content += `*No tables in this schema.*

`;
          continue;
        }

        for (const [tableName, columns] of Object.entries(tables)) {
          const fullTableRef = `${dt.datatable_name}/${schemaName}.${tableName}`;
          content += `**Table: \`${schemaName}.${tableName}\`** (ref: \`${fullTableRef}\`)

| Column | Type |
|--------|------|
`;

          for (const [colName, colType] of Object.entries(columns)) {
            content += `| ${colName} | \`${colType}\` |
`;
          }

          content += `
`;
        }
      }
    }
  }

  // Add usage section
  content += `## Using Datatables in Your App

### Backend Scripts

To query datatables from backend scripts:

\`\`\`typescript
// TypeScript backend script
import * as wmill from "windmill-client";

export async function main() {
    // Get a database connection
    const db = await wmill.getResource("datatable://YOUR_DATATABLE");

    // Run a query
    const result = await db.query("SELECT * FROM schema.table_name WHERE condition = $1", [param]);

    return result;
}
\`\`\`

\`\`\`python
# Python backend script
import wmill

def main():
    # Get a database connection
    db = wmill.get_resource("datatable://YOUR_DATATABLE")

    # Run a query
    result = db.execute("SELECT * FROM schema.table_name WHERE condition = %s", (param,))

    return result
\`\`\`

### Table References in raw_app.yaml

To allow your app to access specific tables, add them to \`data.tables\` in \`raw_app.yaml\`:

\`\`\`yaml
data:
  datatable: main           # Default datatable for new tables
  schema: my_schema         # Default schema for new tables
  tables:
    - main/my_schema.users  # Format: datatable/schema.table
    - main/my_schema.orders
\`\`\`

## SQL Migrations

Place SQL files in the \`sql_to_apply/\` folder. When running \`wmill app dev\`, changes to these files
will trigger a confirmation modal to apply the SQL to the configured datatable.

Example migration file (\`sql_to_apply/001_create_users.sql\`):

\`\`\`sql
CREATE TABLE IF NOT EXISTS my_schema.users (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
\`\`\`

**Important**: After creating tables, add them to \`data.tables\` in \`raw_app.yaml\`.

---
*This file was generated by \`wmill app generate-datatables\`. Run this command again to update.*
`;

  return content;
}

async function generateDatatables(
  opts: GenerateDatatablesOptions,
  appFolder?: string
) {
  // Resolve the app folder
  const cwd = Deno.cwd();
  let targetDir = cwd;

  if (appFolder) {
    targetDir = path.isAbsolute(appFolder)
      ? appFolder
      : path.join(cwd, appFolder);
  }

  // Ensure we're in a .raw_app folder or targeting one
  const dirName = path.basename(targetDir);
  if (!dirName.endsWith(".raw_app")) {
    // Check if current directory is a .raw_app folder
    if (!path.basename(cwd).endsWith(".raw_app") && !appFolder) {
      log.error(
        colors.red(
          "Error: Must be run inside a .raw_app folder or specify one as argument."
        )
      );
      log.info(colors.gray("Usage: wmill app generate-datatables [app_folder]"));
      Deno.exit(1);
    }
  }

  // Check for raw_app.yaml
  const rawAppPath = path.join(targetDir, "raw_app.yaml");
  if (!fs.existsSync(rawAppPath)) {
    log.error(
      colors.red(
        `Error: raw_app.yaml not found in ${targetDir}`
      )
    );
    Deno.exit(1);
  }

  // Resolve workspace and authenticate
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const workspaceId = workspace.workspaceId;

  log.info(colors.cyan("Fetching datatable schemas from workspace..."));

  // Fetch schemas
  let schemas: DataTableSchema[] = [];
  try {
    schemas = await wmill.listDataTableSchemas({ workspace: workspaceId });
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : String(error);
    log.warn(
      colors.yellow(`Could not fetch datatable schemas: ${errorMessage}`)
    );
  }

  // Read local data configuration from raw_app.yaml
  let localData: { tables?: string[]; datatable?: string; schema?: string } | undefined;
  try {
    const rawApp = (await yamlParseFile(rawAppPath)) as Record<string, unknown>;
    if (rawApp.data && typeof rawApp.data === "object") {
      localData = rawApp.data as typeof localData;
    }
  } catch {
    // Ignore errors reading raw_app.yaml
  }

  // Generate the markdown content
  const markdownContent = generateDatatablesMarkdown(schemas, localData);

  // Write to file
  const outputPath = opts.output || path.join(targetDir, "DATATABLES.md");
  await Deno.writeTextFile(outputPath, markdownContent);

  log.info(colors.green(`✓ Generated ${outputPath}`));

  // Summary
  const datatableCount = schemas.length;
  const tableCount = schemas.reduce((acc, dt) => {
    if (!dt.schemas) return acc;
    return (
      acc +
      Object.values(dt.schemas).reduce((schemaAcc, tables) => {
        return schemaAcc + Object.keys(tables || {}).length;
      }, 0)
    );
  }, 0);

  log.info(
    colors.gray(
      `  Found ${datatableCount} datatable(s) with ${tableCount} table(s)`
    )
  );

  if (localData?.datatable) {
    log.info(
      colors.gray(`  App configured for datatable: ${localData.datatable}`)
    );
  }
}

// deno-lint-ignore no-explicit-any
const command = new Command()
  .description("regenerate DATATABLES.md from remote workspace datatable schemas")
  .arguments("[app_folder:string]")
  .option("-o, --output <path:string>", "Output file path (default: DATATABLES.md in app folder)")
  .action(generateDatatables as any);

export default command;
