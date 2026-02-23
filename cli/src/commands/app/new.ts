import { stat, writeFile, mkdir } from "node:fs/promises";
import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import { Input } from "@cliffy/prompt/input";
import { Select } from "@cliffy/prompt/select";
import * as log from "../../core/log.ts";
import { stringify as yamlStringify } from "yaml";
import { GlobalOptions } from "../../types.ts";
import { generateAgentsDocumentation, generateDatatablesDocumentation, yamlOptions } from "../sync/sync.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import * as wmill from "../../../gen/services.gen.ts";
import path from "node:path";
import {
  buildFolderPath,
  loadNonDottedPathsSetting,
} from "../../utils/resource_folders.ts";

// Framework templates - adapted from frontend/src/routes/(root)/(logged)/apps_raw/add/templates.ts
const reactIndex = `
import React from 'react'

import { createRoot } from 'react-dom/client'
import App from './App'

const root = createRoot(document.getElementById('root')!);
root.render(<App/>);
`;

const appTsx = `import React, { useState } from 'react'
import { backend } from './wmill'
import './index.css'

const App = () => {
    const [value, setValue] = useState(undefined as string | undefined)
    const [loading, setLoading] = useState(false)

    async function runA() {
        setLoading(true)
        try {
            setValue(await backend.a({ x: 42 }))
        } catch (e) {
            console.error()
        }
        setLoading(false)
    }

    return <div style={{ width: "100%" }}>
        <h1>hello world</h1>

        <button style={{ marginTop: "2px" }} onClick={runA}>Run 'a'</button>

        <div style={{ marginTop: "20px", width: '250px' }} className='myclass'>
            {loading ? 'Loading ...' : value ?? 'Click button to see value here'}
        </div>
    </div>;
};

export default App;
`;

const appSvelte = `<style>
  h1 {
    font-size: 1.5rem;
  }
</style>

<main>
  <h1>Hello {name}</h1>
</main>

<script>
  let name = 'world';
</script>`;

const indexSvelte = `
import { mount } from 'svelte';
import App from './App.svelte'
import './index.css'

const app = mount(App, { target: document.getElementById("root")! });

export default app;
`;

const appVue = `<template>
  <h1>Hello {{ msg }}</h1>
</template>

<script setup>
import { ref } from 'vue';
const msg = ref('world');
</script>`;

const indexVue = `import { createApp } from 'vue'
import App from './App.vue'
import "./index.css";

createApp(App).mount('#root')`;

const indexCss = `.myclass {
    border: 1px solid gray;
    padding: 2px;
}`;

interface FrameworkTemplate {
  name: string;
  files: Record<string, string>;
}

const templates: Record<string, FrameworkTemplate> = {
  react19: {
    name: "React 19",
    files: {
      "/index.tsx": reactIndex,
      "/App.tsx": appTsx,
      "/index.css": indexCss,
      "/package.json": `{
    "dependencies": {
        "react": "19.0.0",
        "react-dom": "19.0.0",
        "windmill-client": "^1"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`,
    },
  },
  react18: {
    name: "React 18",
    files: {
      "/index.tsx": reactIndex,
      "/App.tsx": appTsx,
      "/index.css": indexCss,
      "/package.json": `{
    "dependencies": {
        "react": "18.3.1",
        "react-dom": "18.3.1",
        "windmill-client": "^1"
    },
    "devDependencies": {
        "@types/react-dom": "^19.0.0",
        "@types/react": "^19.0.0"
    }
}`,
    },
  },
  svelte5: {
    name: "Svelte 5",
    files: {
      "/index.ts": indexSvelte,
      "/App.svelte": appSvelte,
      "/index.css": indexCss,
      "/package.json": `{
    "dependencies": {
        "svelte": "5.45.2",
        "windmill-client": "^1"
    }
}`,
    },
  },
  vue: {
    name: "Vue 3",
    files: {
      "/index.ts": indexVue,
      "/App.vue": appVue,
      "/index.css": indexCss,
      "/package.json": `{
    "dependencies": {
        "core-js": "3.26.1",
        "vue": "3.5.13",
        "windmill-client": "^1"
    }
}`,
    },
  },
};

/**
 * Validates that a path follows the Windmill app path conventions:
 * - Must start with u/ (user) or f/ (folder)
 * - Cannot contain dots
 * - Can only contain alphanumeric characters, underscores, hyphens, and slashes
 */
function validateAppPath(appPath: string): { valid: boolean; error?: string } {
  if (!appPath.startsWith("u/") && !appPath.startsWith("f/")) {
    return {
      valid: false,
      error: "Path must start with 'u/' (user) or 'f/' (folder)",
    };
  }

  if (appPath.includes(".")) {
    return {
      valid: false,
      error: "Path cannot contain dots (.)",
    };
  }

  // Check for valid characters: alphanumeric, underscore, hyphen, and slash
  const validPathRegex = /^[a-zA-Z0-9_\-/]+$/;
  if (!validPathRegex.test(appPath)) {
    return {
      valid: false,
      error:
        "Path can only contain alphanumeric characters, underscores, hyphens, and slashes",
    };
  }

  // Check that path has at least two segments (prefix/name)
  const segments = appPath.split("/").filter((s) => s.length > 0);
  if (segments.length < 2) {
    return {
      valid: false,
      error: "Path must have at least two segments (e.g., u/username/app_name)",
    };
  }

  return { valid: true };
}

/**
 * Generate a unique schema name (appX where X is first unused number)
 */
function generateUniqueSchemaName(existingSchemas: string[]): string {
  let num = 1;
  while (existingSchemas.includes(`app${num}`)) {
    num++;
  }
  return `app${num}`;
}

interface DataConfig {
  tables?: string[];
  datatable?: string;
  schema?: string;
}

async function newApp(opts: GlobalOptions) {
  log.info(colors.bold.cyan("Create a new Windmill Raw App"));
  log.info("");

  // Resolve workspace and authenticate
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const workspaceId = workspace.workspaceId;

  // Fetch available folders for autocompletion
  let folderNames: string[] = [];
  try {
    folderNames = await wmill.listFolderNames({ workspace: workspaceId });
  } catch {
    // Ignore errors fetching folders
  }

  // Fetch available datatables
  let datatables: string[] = [];
  let datatableSchemas: Map<string, string[]> = new Map();

  try {
    log.info(colors.gray("Fetching available datatables..."));
    datatables = await wmill.listDataTables({ workspace: workspaceId });

    if (datatables.length > 0) {
      // Fetch schemas for all datatables
      const schemaData = await wmill.listDataTableSchemas({
        workspace: workspaceId,
      });
      for (const dt of schemaData) {
        if (dt.schemas && !dt.error) {
          const schemaNames = Object.keys(dt.schemas);
          datatableSchemas.set(dt.datatable_name, schemaNames);
        }
      }
    }
  } catch (error: unknown) {
    const errorMessage =
      error instanceof Error ? error.message : String(error);
    log.warn(
      colors.yellow(`Could not fetch datatables: ${errorMessage}`)
    );
  }

  // Ask for summary
  const summary = await Input.prompt({
    message: "App summary (short description):",
    minLength: 1,
    validate: (value: string) => {
      if (value.trim().length === 0) {
        return "Summary cannot be empty";
      }
      return true;
    },
  });

  // Build suggestions for path autocompletion
  const buildPathSuggestions = (input: string): string[] => {
    const suggestions: string[] = [];

    // If empty or very short, suggest prefixes
    if (input.length < 2) {
      suggestions.push("f/", "u/");
    }

    // If starts with f/, suggest folders
    if (input.startsWith("f/")) {
      for (const folder of folderNames) {
        const folderPath = `f/${folder}/`;
        if (folderPath.startsWith(input) && folderPath !== input) {
          suggestions.push(folderPath);
        }
      }
    }

    return suggestions;
  };

  // Ask for path with validation
  let appPath: string;
  while (true) {
    appPath = await Input.prompt({
      message: "App path (e.g., f/my_folder/my_app or u/username/my_app):",
      minLength: 1,
      suggestions: buildPathSuggestions,
    });

    const validation = validateAppPath(appPath);
    if (validation.valid) {
      break;
    }
    log.error(colors.red(`Invalid path: ${validation.error}`));
  }

  // Ask for framework
  const framework = await Select.prompt({
    message: "Select a framework:",
    options: [
      { name: "React 19 (Recommended)", value: "react19" },
      { name: "React 18", value: "react18" },
      { name: "Svelte 5", value: "svelte5" },
      { name: "Vue 3", value: "vue" },
    ],
  });

  const template = templates[framework];
  if (!template) {
    log.error(colors.red(`Unknown framework: ${framework}`));
    return;
  }

  // Data configuration
  let dataConfig: DataConfig = {};
  let createSchemaSQL: string | undefined;
  let schemaName: string | undefined;

  if (datatables.length > 0) {
    log.info("");
    log.info(colors.bold.cyan("Data Configuration"));
    log.info(
      colors.gray(
        "Configure datatables to enable AI to create and query database tables."
      )
    );

    // Ask if user wants to configure datatables
    const configureDatatables = await Confirm.prompt({
      message: "Configure datatable for this app?",
      default: true,
    });

    if (configureDatatables) {
      // Select datatable
      const datatableOptions = datatables.map((dt) => ({
        name: dt === "main" ? `${dt} (Recommended)` : dt,
        value: dt,
      }));

      // Put "main" first if it exists
      datatableOptions.sort((a, b) => {
        if (a.value === "main") return -1;
        if (b.value === "main") return 1;
        return a.value.localeCompare(b.value);
      });

      const selectedDatatable = await Select.prompt({
        message: "Select a datatable:",
        options: datatableOptions,
      });

      dataConfig.datatable = selectedDatatable;

      // Get existing schemas for this datatable
      const existingSchemas = datatableSchemas.get(selectedDatatable) || [];

      // Ask for schema mode
      const schemaOptions: { name: string; value: string }[] = [
        { name: "Create new schema (Recommended for new apps)", value: "new" },
      ];

      if (existingSchemas.length > 0) {
        schemaOptions.push({
          name: "Use existing schema",
          value: "existing",
        });
      }

      schemaOptions.push({ name: "No schema (use public)", value: "none" });

      const schemaMode = await Select.prompt({
        message: "Schema configuration:",
        options: schemaOptions,
      });

      if (schemaMode === "new") {
        // Generate unique schema name
        const defaultSchemaName = generateUniqueSchemaName(existingSchemas);

        schemaName = await Input.prompt({
          message: "New schema name:",
          default: defaultSchemaName,
          validate: (value: string) => {
            if (!value.trim()) {
              return "Schema name cannot be empty";
            }
            if (!/^[a-z_][a-z0-9_]*$/.test(value)) {
              return "Schema name must start with a letter or underscore and contain only lowercase letters, numbers, and underscores";
            }
            if (existingSchemas.includes(value)) {
              return `Schema "${value}" already exists`;
            }
            return true;
          },
        });

        dataConfig.schema = schemaName;

        // Generate SQL to create the schema
        createSchemaSQL = `-- Create schema for ${summary}
-- This will be executed when you run 'wmill app dev' and confirm in the modal
CREATE SCHEMA IF NOT EXISTS ${schemaName};
`;
      } else if (schemaMode === "existing") {
        const schemaSelectOptions = existingSchemas.map((s) => ({
          name: s,
          value: s,
        }));

        schemaName = await Select.prompt({
          message: "Select existing schema:",
          options: schemaSelectOptions,
        });

        dataConfig.schema = schemaName;
      }
      // For "none", we don't set a schema

      dataConfig.tables = [];
    }
  } else {
    log.info("");
    log.info(
      colors.yellow(
        "No datatables configured in this workspace. Skipping data configuration."
      )
    );
    log.info(
      colors.gray(
        "You can configure datatables in Workspace Settings > Windmill Data Tables"
      )
    );
  }

  // Load nonDottedPaths setting from wmill.yaml before creating folder
  await loadNonDottedPathsSetting();

  // Create the directory structure - preserve full path (e.g., f/foobar/x/y becomes f/foobar/x/y.raw_app)
  const folderName = buildFolderPath(appPath, "raw_app");
  const appDir = path.join(process.cwd(), folderName);

  // Check if directory already exists
  try {
    await stat(appDir);
    const overwrite = await Confirm.prompt({
      message: `Directory '${folderName}' already exists. Overwrite?`,
      default: false,
    });
    if (!overwrite) {
      log.info(colors.yellow("Aborted."));
      return;
    }
  } catch {
    // Directory doesn't exist, which is good
  }

  await mkdir(appDir, { recursive: true });
  await mkdir(path.join(appDir, "backend"), { recursive: true });
  await mkdir(path.join(appDir, "sql_to_apply"), { recursive: true });

  // Create raw_app.yaml with data configuration
  const rawAppConfig: Record<string, unknown> = {
    summary,
  };

  // Add data configuration if present
  if (dataConfig.datatable) {
    rawAppConfig.data = dataConfig;
  }

  await writeFile(
    path.join(appDir, "raw_app.yaml"),
    yamlStringify(rawAppConfig, yamlOptions), "utf-8"
  );

  // Create template files
  for (const [filePath, content] of Object.entries(template.files)) {
    const fullPath = path.join(appDir, filePath.slice(1)); // Remove leading slash
    await writeFile(fullPath, content.trim() + "\n", "utf-8");
  }

  // Create AGENTS.md - main documentation for AI agents
  const dataForDocs = dataConfig.datatable
    ? {
        tables: dataConfig.tables,
        datatable: dataConfig.datatable,
        schema: dataConfig.schema,
      }
    : undefined;

  const agentsContent = generateAgentsDocumentation(dataForDocs);
  await writeFile(
    path.join(appDir, "AGENTS.md"),
    agentsContent, "utf-8"
  );

  // Create CLAUDE.md referencing AGENTS.md
  await writeFile(
    path.join(appDir, "CLAUDE.md"),
    `Instructions are in @AGENTS.md\n`, "utf-8"
  );

  // Create DATATABLES.md with the configured data
  const datatablesContent = generateDatatablesDocumentation(dataForDocs);
  await writeFile(
    path.join(appDir, "DATATABLES.md"),
    datatablesContent, "utf-8"
  );

  // Create example backend runnable
  const exampleRunnable = {
    type: "inline",
    path: undefined,
  };
  await writeFile(
    path.join(appDir, "backend", "a.yaml"),
    yamlStringify(exampleRunnable, yamlOptions), "utf-8"
  );
  await writeFile(
    path.join(appDir, "backend", "a.ts"),
    `export async function main(x: number): Promise<string> {
  return \`Hello from backend! x = \${x}\`;
}
`, "utf-8"
  );

  // Create sql_to_apply README
  await writeFile(
    path.join(appDir, "sql_to_apply", "README.md"),
    `# SQL Migrations Folder

This folder is for SQL migration files that will be applied to datatables during development.

## How to Use

1. Configure a datatable in \`raw_app.yaml\`:
   \`\`\`yaml
   data:
     datatable: main  # Your datatable name
     tables: []       # Add tables here after creating them
   \`\`\`

2. Create SQL files in this folder (e.g., \`001_create_users.sql\`)

3. Run \`wmill app dev .\` - the dev server watches this folder

4. When a SQL file is created/modified, a modal appears to confirm execution

5. After creating tables, add them to \`data.tables\` in \`raw_app.yaml\`

## Important

- This folder is **excluded from push** - SQL files are not synced
- Always add created tables to \`data.tables\` so your app can access them
- Use idempotent SQL (\`CREATE TABLE IF NOT EXISTS\`, etc.)
`
  );

  // Create schema creation SQL file if a new schema was requested
  if (createSchemaSQL && schemaName) {
    await writeFile(
      path.join(appDir, "sql_to_apply", `000_create_schema_${schemaName}.sql`),
      createSchemaSQL, "utf-8"
    );
  }

  log.info("");
  log.info(colors.bold.green(`App created successfully at ${folderName}/`));
  log.info("");
  log.info(colors.gray("Directory structure:"));
  log.info(colors.gray(`  ${folderName}/`));
  log.info(colors.gray("  ├── AGENTS.md        ← Read this first!"));
  log.info(colors.gray("  ├── raw_app.yaml"));
  log.info(colors.gray("  ├── DATATABLES.md"));
  for (const filePath of Object.keys(template.files)) {
    log.info(colors.gray(`  ├── ${filePath.slice(1)}`));
  }
  log.info(colors.gray("  ├── backend/"));
  log.info(colors.gray("  │   ├── a.yaml"));
  log.info(colors.gray("  │   └── a.ts"));
  log.info(colors.gray("  └── sql_to_apply/"));
  log.info(colors.gray("      ├── README.md"));
  if (createSchemaSQL && schemaName) {
    log.info(
      colors.gray(`      └── 000_create_schema_${schemaName}.sql`)
    );
  }

  log.info("");

  // Show data configuration summary
  if (dataConfig.datatable) {
    log.info(colors.cyan("Data configuration:"));
    log.info(colors.gray(`  Datatable: ${dataConfig.datatable}`));
    if (dataConfig.schema) {
      log.info(colors.gray(`  Schema: ${dataConfig.schema}`));
    }
    log.info("");
  }

  // Advise to run wmill app dev
  log.info(colors.bold.cyan("Next steps:"));
  log.info(colors.gray(`  1. cd ${folderName}`));
  log.info(colors.gray("  2. npm install"));
  log.info(
    colors.bold.white("  3. wmill app dev .") +
      colors.gray(" (start dev server)")
  );
  if (createSchemaSQL) {
    log.info("");
    log.info(
      colors.yellow(
        `     A SQL file to create schema '${schemaName}' has been added to sql_to_apply/`
      )
    );
    log.info(
      colors.yellow(
        "     The dev server will prompt you to apply it when you start."
      )
    );
  }
  log.info("");
  log.info(colors.gray("  4. wmill sync push (to deploy when ready)"));
}

const command = new Command()
  .description("create a new raw app from a template")
  .action(newApp as any);

export default command;
