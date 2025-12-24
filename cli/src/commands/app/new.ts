import {
  colors,
  Command,
  Confirm,
  ensureDir,
  Input,
  log,
  Select,
  yamlStringify,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { generateDatatablesDocumentation, yamlOptions } from "../sync/sync.ts";
import path from "node:path";

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

async function newApp(_opts: GlobalOptions) {
  log.info(colors.bold.cyan("Create a new Windmill Raw App"));
  log.info("");

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

  // Ask for path with validation
  let appPath: string;
  while (true) {
    appPath = await Input.prompt({
      message: "App path (e.g., f/my_folder/my_app or u/username/my_app):",
      minLength: 1,
      suggestions: ["f/", "u/"],
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

  // Create the directory structure
  const folderName = `${appPath.split("/").pop()}.raw_app`;
  const appDir = path.join(Deno.cwd(), folderName);

  // Check if directory already exists
  try {
    await Deno.stat(appDir);
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

  await ensureDir(appDir);
  await ensureDir(path.join(appDir, "backend"));
  await ensureDir(path.join(appDir, "sql_to_apply"));

  // Create raw_app.yaml
  const rawAppConfig = {
    summary,
    custom_path: appPath,
  };
  await Deno.writeTextFile(
    path.join(appDir, "raw_app.yaml"),
    yamlStringify(rawAppConfig, yamlOptions)
  );

  // Create template files
  for (const [filePath, content] of Object.entries(template.files)) {
    const fullPath = path.join(appDir, filePath.slice(1)); // Remove leading slash
    await Deno.writeTextFile(fullPath, content.trim() + "\n");
  }

  // Create DATATABLES.md
  const datatablesContent = generateDatatablesDocumentation(undefined);
  await Deno.writeTextFile(
    path.join(appDir, "DATATABLES.md"),
    datatablesContent
  );

  // Create example backend runnable
  const exampleRunnable = {
    type: "inline",
    path: undefined,
  };
  await Deno.writeTextFile(
    path.join(appDir, "backend", "a.yaml"),
    yamlStringify(exampleRunnable, yamlOptions)
  );
  await Deno.writeTextFile(
    path.join(appDir, "backend", "a.ts"),
    `export async function main(x: number): Promise<string> {
  return \`Hello from backend! x = \${x}\`;
}
`
  );

  // Create sql_to_apply README
  await Deno.writeTextFile(
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

  log.info("");
  log.info(colors.bold.green(`App created successfully at ${folderName}/`));
  log.info("");
  log.info(colors.gray("Directory structure:"));
  log.info(colors.gray(`  ${folderName}/`));
  log.info(colors.gray("  ├── raw_app.yaml"));
  log.info(colors.gray("  ├── DATATABLES.md"));
  for (const filePath of Object.keys(template.files)) {
    log.info(colors.gray(`  ├── ${filePath.slice(1)}`));
  }
  log.info(colors.gray("  ├── backend/"));
  log.info(colors.gray("  │   ├── a.yaml"));
  log.info(colors.gray("  │   └── a.ts"));
  log.info(colors.gray("  └── sql_to_apply/"));
  log.info(colors.gray("      └── README.md"));
  log.info("");
  log.info(colors.cyan("Next steps:"));
  log.info(colors.gray(`  1. cd ${folderName}`));
  log.info(colors.gray("  2. npm install"));
  log.info(colors.gray("  3. wmill app dev ."));
  log.info(colors.gray("  4. wmill sync push (to deploy)"));
}

// deno-lint-ignore no-explicit-any
const command = new Command()
  .description("create a new raw app from a template")
  .action(newApp as any);

export default command;
