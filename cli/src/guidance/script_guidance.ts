// CLI Script Guidance - Uses centralized prompts from system_prompts/
import * as prompts from "./prompts.ts";

// CLI-specific introduction
const CLI_INTRO = `Each script should be placed in a folder. Ask the user in which folder he wants the script to be located at before starting coding.
After writing a script, you do not need to create .lock and .yaml files manually. Instead, you can run \`wmill script generate-metadata\` bash command. This command takes no arguments. After writing the script, you can ask the user if he wants to push the script with \`wmill sync push\`. Both should be run at the root of the repository.

You can use \`wmill resource-type list --schema\` to list all resource types available. You should use that to know the type of the resource you need to use in your script. You can use grep if the output is too long.`;

// Combine all language prompts
const LANGUAGE_PROMPTS = [
  prompts.LANG_BUN,
  prompts.LANG_DENO,
  prompts.LANG_PYTHON3,
  prompts.LANG_GO,
  prompts.LANG_BASH,
  prompts.LANG_POSTGRESQL,
  prompts.LANG_MYSQL,
  prompts.LANG_BIGQUERY,
  prompts.LANG_SNOWFLAKE,
  prompts.LANG_MSSQL,
  prompts.LANG_GRAPHQL,
  prompts.LANG_POWERSHELL,
  prompts.LANG_RUST,
  prompts.LANG_PHP,
  prompts.LANG_CSHARP,
  prompts.LANG_JAVA,
  prompts.LANG_NATIVETS,
  prompts.LANG_BUNNATIVE,
  prompts.LANG_DUCKDB,
]
  .filter(Boolean)
  .join("\n\n");

// Assemble complete script guidance
export const SCRIPT_GUIDANCE = `
${CLI_INTRO}

${prompts.SCRIPT_BASE}

${LANGUAGE_PROMPTS}

${prompts.S3_OBJECTS}

${prompts.SDK_TYPESCRIPT}

${prompts.SDK_PYTHON}
`;
