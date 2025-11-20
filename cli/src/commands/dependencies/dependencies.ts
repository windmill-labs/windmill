// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";
import { colors, Command, log } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import type { ScriptLang, NewWorkspaceDependencies } from "../../../gen/types.gen.ts";
import fs from "node:fs";
import { generateHash } from "../../utils/utils.ts";
import { checkifMetadataUptodate, updateMetadataGlobalLock } from "../../utils/metadata.ts";

async function push(
  opts: GlobalOptions,
  filePath: string,
  language?: ScriptLang,
  name?: string
): Promise<void> {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`);
  }

  const content = fs.readFileSync(filePath, "utf8");

  // Use the existing pushWorkspaceDependencies function
  await pushWorkspaceDependencies(workspace.workspaceId, filePath, null, content);
}

async function add(
  opts: GlobalOptions,
  language: ScriptLang,
  name?: string
): Promise<void> {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Default templates for different languages
  const templates: Partial<Record<ScriptLang, string>> = {
    'python3': `# Python Requirements (requirements.in format)
## py: 3.11
#^ Uncomment to pin to python version.
# Core dependencies
requests>=2.31.0
pandas>=2.0.0
numpy>=1.24.0

# Add your dependencies here
`,
    'nativets': `{
  "name": "windmill-typescript-script",
  "version": "1.0.0",
  "description": "TypeScript script dependencies for Windmill",
  "dependencies": {
    "axios": "^1.6.0",
    "lodash": "^4.17.21",
    "date-fns": "^2.30.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  }
}`,
    'go': `module windmill-go-script

go 1.21

require (
    github.com/gorilla/mux v1.8.1
    github.com/lib/pq v1.10.9
)`,
    'php': `{
  "name": "windmill-php-script",
  "description": "PHP script dependencies for Windmill",
  "require": {
    "php": ">=8.1",
    "guzzlehttp/guzzle": "^7.8",
    "monolog/monolog": "^3.5"
  }
}`
  };

  const template = templates[language];
  if (!template) {
    throw new Error(`No template available for language: ${language}`);
  }

  const dependenciesData: DependenciesFile = {
    name,
    content: template,
    language,
    description: `Dependencies for ${language}${name ? ` - ${name}` : ' (workspace default)'}`
  };

  log.info(colors.yellow(`Creating ${name ? 'named' : 'workspace default'} dependencies for ${language}...`));

  try {
    await wmill.createWorkspaceDependencies({
      workspace: workspace.workspaceId,
      requestBody: {
        name: dependenciesData.name || undefined,
        content: dependenciesData.content,
        language: dependenciesData.language,
        workspace_id: workspace.workspaceId
      }
    });

    const displayName = name ? `named dependencies "${name}"` : `workspace default dependencies`;
    log.info(colors.bold.underline.green(`Successfully created ${displayName} for ${language}`));
    log.info(colors.gray(`You can now edit the dependencies in the Windmill UI or push an updated file.`));
  } catch (error: any) {
    log.error(colors.red(`Failed to create dependencies: ${error.message}`));
    throw error;
  }
}

const command = new Command()
  .alias("deps")
  .description("workspace dependencies related commands")
  .command(
    "push",
    "Push workspace dependencies from a local file"
  )
  .arguments("<file_path:string>")
  .option(
    "--language <language:string>",
    "Programming language (python3, typescript, go, php). If not specified, will be inferred from file extension."
  )
  .option(
    "--name <name:string>",
    "Name for the dependencies. If not specified, creates workspace default dependencies."
  )
  .action(push as any)
  .command(
    "add",
    "Create new workspace dependencies with a default template"
  )
  .arguments("<language:string>")
  .option(
    "--name <name:string>",
    "Name for the dependencies. If not specified, creates workspace default dependencies."
  )
  .action(add as any);

export async function pushWorkspaceDependencies(
  workspace: string,
  path: string,
  _befObj: any,
  newDependenciesContent: string
): Promise<void> {
  try {
    // Parse the workspace dependencies file path
    // Format: dependencies/requirements.in, dependencies/package.json, dependencies/myname.requirements.in, etc.
    const relativePath = path.replace("dependencies/", "");
    
    let language: ScriptLang;
    let name: string | undefined;
    
    // Parse based on file extension and potential name prefix
    if (relativePath.endsWith("requirements.in")) {
      language = "python3";
      name = relativePath === "requirements.in" ? undefined : relativePath.replace(".requirements.in", "");
    } else if (relativePath.endsWith("package.json")) {
      // Could be Bun, Deno, or Nativets - we'll default to nativets for sync
      language = "nativets";
      name = relativePath === "package.json" ? undefined : relativePath.replace(".package.json", "");
    } else if (relativePath.endsWith("go.mod")) {
      language = "go";
      name = relativePath === "go.mod" ? undefined : relativePath.replace(".go.mod", "");
    } else if (relativePath.endsWith("composer.json")) {
      language = "php";
      name = relativePath === "composer.json" ? undefined : relativePath.replace(".composer.json", "");
    } else {
      throw new Error(`Unknown workspace dependencies file format: ${path}`);
    }

    // Generate hash for workspace dependencies content and metadata
    const contentHash = await generateHash(
      newDependenciesContent + 
      language + 
      (name || "")
    );

    // Check if dependencies are up-to-date using wmill-lock.yaml tracking
    const isUpToDate = await checkifMetadataUptodate(path, contentHash, undefined);
    
    if (isUpToDate) {
      const displayName = name ? `named dependencies "${name}"` : `workspace default dependencies`;
      log.info(colors.green(`${displayName} for ${language} are up-to-date, skipping push`));
      return;
    }

    log.info(colors.yellow(`Pushing ${name ? 'named' : 'workspace default'} dependencies for ${language}...`));

    await wmill.createWorkspaceDependencies({
      workspace,
      requestBody: {
        name,
        content: newDependenciesContent,
        language,
        workspace_id: workspace,
        // Description is not supported in cli, it will use old description
        description: undefined
      }
    });

    // Update wmill-lock.yaml with new hash after successful push
    await updateMetadataGlobalLock(path, contentHash);

    const displayName = name ? `named dependencies "${name}"` : `workspace default dependencies`;
    log.info(colors.green(`Successfully pushed ${displayName} for ${language}`));
  } catch (error: any) {
    log.error(colors.red(`Failed to push workspace dependencies: ${error.message}`));
    throw error;
  }
}

export default command;
