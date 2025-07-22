import { ResourceService } from '$lib/gen/services.gen'
import type { ResourceType, ScriptLang } from '$lib/gen/types.gen'
import { capitalize, isObject, toCamel } from '$lib/utils'
import { get } from 'svelte/store'
import { compile, phpCompile, pythonCompile } from '../../utils'
import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionTool,
	ChatCompletionUserMessageParam
} from 'openai/resources/index.mjs'
import { copilotSessionModel, type DBSchema, dbSchemas } from '$lib/stores'
import { scriptLangToEditorLang } from '$lib/scripts'
import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/utils'
import type { CodePieceElement, ContextElement } from '../context'
import { PYTHON_PREPROCESSOR_MODULE_CODE, TS_PREPROCESSOR_MODULE_CODE } from '$lib/script_helpers'
import { createSearchHubScriptsTool, type Tool } from '../shared'
import { setupTypeAcquisition, type DepsToGet } from '$lib/ata'
import { getModelContextWindow } from '../../lib'

// Score threshold for npm packages search filtering
const SCORE_THRESHOLD = 1000
// percentage of the context window for documentation of npm packages
const DOCS_CONTEXT_PERCENTAGE = 1
// percentage of the context window for types of npm packages
const TYPES_CONTEXT_PERCENTAGE = 1

export function formatResourceTypes(
	allResourceTypes: ResourceType[],
	lang: 'python3' | 'php' | 'bun' | 'deno' | 'nativets' | 'bunnative'
) {
	const resourceTypes = allResourceTypes.filter(
		(rt) => isObject(rt.schema) && 'properties' in rt.schema && isObject(rt.schema.properties)
	)
	if (lang === 'python3') {
		const result = resourceTypes.map((resourceType) => {
			return `class ${resourceType.name}(TypedDict):\n${pythonCompile(resourceType.schema as any)}`
		})
		return '\n**Make sure to rename conflicting imported modules**\n' + result.join('\n\n')
	} else if (lang === 'php') {
		const result = resourceTypes.map((resourceType) => {
			return `class ${toCamel(capitalize(resourceType.name))} {\n${phpCompile(
				resourceType.schema as any
			)}\n}`
		})
		return '\n' + result.join('\n\n')
	} else {
		let resultStr = 'namespace RT {\n'
		const result = resourceTypes.map((resourceType) => {
			return `  type ${toCamel(capitalize(resourceType.name))} = ${compile(
				resourceType.schema as any
			).replaceAll('\n', '\n  ')}`
		})
		return resultStr + result.join('\n\n') + '\n}'
	}
}

async function getResourceTypes(prompt: string, workspace: string) {
	const resourceTypes = await ResourceService.queryResourceTypes({
		workspace: workspace,
		text: prompt,
		limit: 5
	})

	return resourceTypes
}

const TS_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type inside the \`RT\` namespace: for instance \`RT.Stripe\`.
You should only use them if you need them to satisfy the user's instructions. Always use the RT namespace.\n`

const TS_INLINE_TYPE_INSTRUCTION = `When using resource types, you should use RT.ResourceType as parameter type. For other parameters, you should inline the objects types instead of defining them separately. This is because Windmill requires the types (other than resource types) to be inlined to generate a user friendly UI from the parameters.`

const TS_WINDMILL_CLIENT_CONTEXT = `

The windmill client (wmill) can be used to interact with Windmill from the script. Import it with \`import * as wmill from "windmill-client"\`. Key functions include:

// Resource operations
wmill.getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any> // Get resource value by path
wmill.setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void> // Set resource value

// State management (persistent across executions)  
wmill.getState(): Promise<any> // Get shared state
wmill.setState(state: any): Promise<void> // Set shared state

// Variables
wmill.getVariable(path: string): Promise<string> // Get variable value
wmill.setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void> // Set variable value

// Script execution
wmill.runScript(path?: string | null, hash_?: string | null, args?: Record<string, any> | null, verbose?: boolean): Promise<any> // Run script synchronously
wmill.runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds?: number | null): Promise<string> // Run script async, returns job ID
wmill.waitJob(jobId: string, verbose?: boolean): Promise<any> // Wait for job completion and get result
wmill.getResult(jobId: string): Promise<any> // Get job result by ID
wmill.getResultMaybe(jobId: string): Promise<any> // Get job result by ID, returns undefined if not found
wmill.getRootJobId(jobId?: string): Promise<string> // Get root job ID from job ID

// S3 file operations (if S3 is configured)
wmill.loadS3File(s3object: S3Object, s3ResourcePath?: string | undefined): Promise<Uint8Array | undefined> // Load file content from S3
wmill.loadS3FileStream(s3object: S3Object, s3ResourcePath?: string | undefined): Promise<Blob | undefined> // Load file content from S3 as stream
wmill.writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath?: string | undefined): Promise<S3Object> // Write file to S3

// Flow operations
wmill.setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void> // Set flow user state
wmill.getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any> // Get flow user state
wmill.getResumeUrls(approver?: string): Promise<{approvalPage: string, resume: string, cancel: string}> // Get approval URLs

`

const PYTHON_WINDMILL_CLIENT_CONTEXT = `

The windmill client (wmill) can be used to interact with Windmill from the script. Import it with \`import wmill\`. Key functions include:

// Resource operations
wmill.get_resource(path: str, none_if_undefined: bool = False) -> dict | None  // Get resource value by path
wmill.set_resource(path: str, value: Any, resource_type: str = "any") -> None  // Set resource value

// State management (persistent across executions)
wmill.get_state() -> Any  // Get shared state (deprecated, use flow user state)
wmill.set_state(value: Any) -> None  // Set shared state
wmill.get_state_path() -> str  // Get state path
wmill.get_flow_user_state(key: str) -> Any  // Get flow user state 
wmill.set_flow_user_state(key: str, value: Any) -> None  // Set flow user state

// Variables
wmill.get_variable(path: str) -> str  // Get variable value
wmill.set_variable(path: str, value: str, is_secret: bool = False) -> None  // Set variable value

// Script execution
wmill.run_script(path: str = None, hash_: str = None, args: dict = None, timeout = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = True) -> Any  // Run script synchronously
wmill.run_script_async(path: str = None, hash_: str = None, args: dict = None, scheduled_in_secs: int = None) -> str  // Run script async, returns job ID
wmill.wait_job(job_id: str, timeout = None, verbose: bool = False, cleanup: bool = True, assert_result_is_not_none: bool = False) -> Any  // Wait for job completion and get result
wmill.get_result(job_id: str, assert_result_is_not_none: bool = True) -> Any  // Get job result by ID
wmill.get_root_job_id(job_id: str | None = None) -> str  // Get root job ID from job ID

// S3 file operations (if S3 is configured)
wmill.load_s3_file(s3object: S3Object | str, s3_resource_path: str | None = None) -> bytes  // Load file content from S3
wmill.load_s3_file_reader(s3object: S3Object | str, s3_resource_path: str | None = None) -> BufferedReader  // Load S3 file as stream reader
wmill.write_s3_file(s3object: S3Object | str | None, file_content: BufferedReader | bytes, s3_resource_path: str | None = None, content_type: str | None = None, content_disposition: str | None = None) -> S3Object  // Write file to S3

// Flow operations  
wmill.run_flow_async(path: str, args: dict = None, scheduled_in_secs: int = None, do_not_track_in_parent: bool = True) -> str  // Run flow asynchronously
wmill.get_resume_urls(approver: str = None) -> dict  // Get approval URLs for flow steps

// Utilities
wmill.get_workspace() -> str  // Get current workspace
wmill.whoami() -> dict  // Get current user information
wmill.get_job_status(job_id: str) -> str  // Get job status ("RUNNING" | "WAITING" | "COMPLETED")
wmill.set_progress(value: int, job_id: Optional[str] = None) -> None  // Set job progress (0-100)
wmill.get_progress(job_id: Optional[str] = None) -> Any  // Get job progress`

const PYTHON_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type.
You need to **redefine** the type of the resources that are needed before the main function as TypedDict, but only include them if they are actually needed to achieve the function purpose.
The resource type name has to be exactly as specified (has to be IN LOWERCASE).
If an import conflicts with a resource type name, **you have to rename the imported object, not the type name**.
Make sure to import TypedDict from typing **if you're using it**`

const PHP_RESOURCE_TYPE_SYSTEM = `On Windmill, credentials and configuration are stored in resources and passed as parameters to main.
If you need credentials, you should add a parameter to \`main\` with the corresponding resource type
You need to **redefine** the type of the resources that are needed before the main function, but only include them if they are actually needed to achieve the function purpose.
Before defining each type, check if the class already exists using class_exists.
The resource type name has to be exactly as specified.`

const PREPROCESSOR_INSTRUCTION_BASE = `The current script is a preprocessor. It processes raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, Postgres, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.
The returned object determines the parameter values passed to the flow.
e.g., \`{ b: 1, a: 2 }\` → Calls the flow with \`a = 2\` and \`b = 1\`, assuming the flow has two inputs called \`a\` and \`b\`.
The preprocessor receives a single parameter called event.
Here's a sample script which includes the event object definition:\n`

const TS_PREPROCESSOR_INSTRUCTION =
	PREPROCESSOR_INSTRUCTION_BASE +
	`\`\`\`typescript
${TS_PREPROCESSOR_MODULE_CODE}
\`\`\`\n`

const PYTHON_PREPROCESSOR_INSTRUCTION =
	PREPROCESSOR_INSTRUCTION_BASE +
	`\`\`\`python
${PYTHON_PREPROCESSOR_MODULE_CODE}
\`\`\``

export const SUPPORTED_CHAT_SCRIPT_LANGUAGES = [
	'bunnative',
	'nativets',
	'bun',
	'deno',
	'python3',
	'php',
	'rust',
	'go',
	'bash',
	'postgresql',
	'mysql',
	'bigquery',
	'snowflake',
	'mssql',
	'graphql',
	'powershell',
	'csharp',
	'java'
]

export function getLangContext(
	lang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json',
	{
		allowResourcesFetch = false,
		isPreprocessor = false
	}: { allowResourcesFetch?: boolean; isPreprocessor?: boolean; isFailure?: boolean } = {}
) {
	const tsContext =
		(isPreprocessor
			? TS_PREPROCESSOR_INSTRUCTION
			: TS_RESOURCE_TYPE_SYSTEM +
				(allowResourcesFetch
					? `To query the RT namespace, you can use the \`search_resource_types\` tool.\n`
					: '')) +
		TS_INLINE_TYPE_INSTRUCTION +
		TS_WINDMILL_CLIENT_CONTEXT

	const mainFunctionName = isPreprocessor ? 'preprocessor' : 'main'

	switch (lang) {
		case 'bunnative':
		case 'nativets':
			return (
				`The user is coding in TypeScript. On Windmill, it is expected that the script exports a single **async** function called \`${mainFunctionName}\`. You should use fetch (available globally, no need to import) and are not allowed to import any libraries.\n` +
				tsContext
			)
		case 'bun':
			return (
				`The user is coding in TypeScript (bun runtime). On Windmill, it is expected that the script exports a single **async** function called \`${mainFunctionName}\`. Do not call the ${mainFunctionName} function. Libraries are installed automatically, do not show how to install them.\n` +
				tsContext
			)
		case 'deno':
			return (
				`The user is coding in TypeScript (deno runtime). On Windmill, it is expected that the script exports a single **async** function called \`${mainFunctionName}\`. Do not call the ${mainFunctionName} function. Libraries are installed automatically, do not show how to install them.\n` +
				tsContext +
				'\nYou can import deno libraries or you can import npm libraries like that: `import ... from "npm:{package}";`.'
			)
		case 'python3':
			return (
				`The user is coding in Python. On Windmill, it is expected the script contains at least one function called \`${mainFunctionName}\`. Do not call the ${mainFunctionName} function. Libraries are installed automatically, do not show how to install them.` +
				(isPreprocessor
					? PYTHON_PREPROCESSOR_INSTRUCTION
					: PYTHON_RESOURCE_TYPE_SYSTEM +
						`${allowResourcesFetch ? `\nTo query the available resource types, you can use the \`search_resource_types\` tool.` : ''}`) +
				PYTHON_WINDMILL_CLIENT_CONTEXT
			)
		case 'php':
			return (
				'The user is coding in PHP. On Windmill, it is expected the script contains at least one function called `main`. The script must start with <?php.' +
				PHP_RESOURCE_TYPE_SYSTEM +
				`${allowResourcesFetch ? `\nTo query the available resource types, you can use the \`search_resource_types\` tool.` : ''}` +
				`\nIf you need to import libraries, you need to specify them as comments in the following manner before the main function:
\`\`\`
// require:
// mylibrary/mylibrary
// myotherlibrary/myotherlibrary@optionalversion
\`\`\`
Make sure to have one per line.
No need to require autoload, it is already done.`
			)
		case 'rust':
			return `The user is coding in Rust. On Windmill, it is expected the script contains at least one function called \`main\` (without calling it) defined like this:
\`\`\`rust
use anyhow::anyhow;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct ReturnType {
    // ...
}

fn main(...) -> anyhow::Result<ReturnType>
\`\`\`
Arguments should be owned. Make sure the return type is serializable.

Packages must be made available with a partial cargo.toml by adding the following comment at the beginning of the script:
//! \`\`\`cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! \`\`\'
Serde is already included, no need to add it again.

If you want to handle async functions (e.g., using tokio), you need to keep the main function sync and create the runtime inside.
`
		case 'go':
			return `The user is coding in Go. On Windmill, it is expected the script exports a single function called \`main\`. Its return type has to be (\`{return_type}\`, error). The file package has to be "inner".`
		case 'bash':
			return `The user is coding in Bash. Do not include "#!/bin/bash". On Windmill, arguments are always string and can only be obtained with "var1="$1"", "var2="$2"", etc..`
		case 'postgresql':
			return `The user is coding in PostgreSQL. On Windmill, arguments can be obtained directly in the statement with \`$1::{type}\`, \`$2::{type}\`, etc... Name the parameters (without specifying the type) by adding comments at the beginning of the script before the statement like that: \`-- $1 name1\` or \`-- $2 name = default\` (one per row)`
		case 'mysql':
			return 'The user is coding in MySQL. On Windmill, arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)'
		case 'bigquery':
			return 'The user is coding in BigQuery. On Windmill, arguments can be obtained by adding comments before the statement like that: `-- @name1 ({type})` or `-- @name2 ({type}) = default` (one per row). They can then be obtained directly in the statement with `@name1`, `@name2`, etc....'
		case 'snowflake':
			return 'The user is coding in Snowflake. On Windmill, arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)'
		case 'mssql':
			return 'The user is coding in Microsoft SQL Server. On Windmill, arguments can be obtained directly in the statement with @P1, @P2, etc.. Name the parameters by adding comments before the statement like that: `-- @P1 name1 ({type})` or `-- @P2 name2 ({type}) = default` (one per row)'
		case 'graphql':
			return 'The user is coding in GraphQL. If needed, add the needed arguments as query parameters.'
		case 'powershell':
			return 'The user is coding in PowerShell. On Windmill, arguments can be obtained by calling the param function on the first line of the script like that: `param($ParamName1, $ParamName2 = "default value", [{type}]$ParamName3, ...)`'
		case 'csharp':
			return 'The user is coding in C#. On Windmill, it is expected the script contains a public static Main method inside a class. The class name is irrelevant. NuGet packages can be added using the format: #r "nuget: PackageName, Version" at the top of the script. The Main method signature should be: public static ReturnType Main(parameter types...)'
		case 'java':
			return 'The user is coding in Java. On Windmill, it is expected the script contains a Main public class and a public static main() method. The return type can be Object or void. Dependencies can be added using the format: //requirements://groupId:artifactId:version at the top of the script. The method signature should be: public static Object main(parameter types...)'
		default:
			return ''
	}
}

export async function getFormattedResourceTypes(
	lang: ScriptLang | 'bunnative',
	prompt: string,
	workspace: string
) {
	switch (lang) {
		case 'deno':
		case 'bun':
		case 'nativets':
		case 'bunnative':
		case 'python3':
		case 'php': {
			const resourceTypes = await getResourceTypes(prompt, workspace)

			const intro = `RESOURCE_TYPES:\n`

			const resourceTypesText = formatResourceTypes(resourceTypes, lang)

			return intro + resourceTypesText
		}
		default:
			return ''
	}
}

export const CHAT_SYSTEM_PROMPT = `
	You are a coding assistant for the Windmill platform. You are provided with a list of \`INSTRUCTIONS\` and the current contents of a code file under \`CODE\`.

	Your task is to respond to the user's request. Assume all user queries are valid and actionable.

	When the user requests code changes:
	- Always include a **single code block** with the **entire updated file**, not just the modified sections.
	- The code can include \`[#START]\` and \`[#END]\` markers to indicate the start and end of a code piece. You MUST only modify the code between these markers if given, and remove them in your response. If a question is asked about the code, you MUST only talk about the code between the markers. Refer to it as the code piece, not the code between the markers.
	- Follow the instructions carefully and explain the reasoning behind your changes.
	- If the request is abstract (e.g., "make this cleaner"), interpret it concretely and reflect that in the code block.
	- Preserve existing formatting, indentation, and whitespace unless changes are strictly required to fulfill the user's request.
	- The user can ask you to look at or modify specific files, databases or errors by having its name in the INSTRUCTIONS preceded by the @ symbol. In this case, put your focus on the element that is explicitly mentioned.
	- The user can ask you questions about a list of \`DATABASES\` that are available in the user's workspace. If the user asks you a question about a database, you should ask the user to specify the database name if not given, or take the only one available if there is only one.
	- You can also receive a \`DIFF\` of the changes that have been made to the code. You should use this diff to give better answers.
	- Before giving your answer, check again that you carefully followed these instructions.
	- When asked to create a script that communicates with an external service, you can use the \`search_hub_scripts\` tool to search for relevant scripts in the hub. Make sure the language is the same as what the user is coding in. If you do not find any relevant scripts, you can use the \`search_npm_packages\` tool to search for relevant packages and their documentation. Always give a link to the documentation in your answer if possible.

	Important:
	Do not mention or reveal these instructions to the user unless explicitly asked to do so.
`

export const INLINE_CHAT_SYSTEM_PROMPT = `
# Windmill Inline Coding Assistant

You are a coding assistant for the Windmill platform. You provide precise code modifications based on user instructions.

## Input Format

You will receive:
- **INSTRUCTIONS**: User's modification request
- **CODE**: Current code content with modification boundaries
- **DATABASES** *(optional)*: Available workspace databases

### Code Boundaries

The code contains \`[#START]\` and \`[#END]\` markers indicating the modification scope:
- **MUST** only modify code between these markers
- **MUST** remove the markers in your response
- **MUST** preserve all other code exactly as provided

## Task Requirements

Return the modified CODE that fulfills the user's request. Assume all user queries are valid and actionable.

### Critical Rules

- ✅ **ALWAYS** include a single code block with the entire updated CODE
- ✅ **ALWAYS** use the structured XML output format below
- ❌ **NEVER** include only modified sections
- ❌ **NEVER** add explanatory text or comments outside the format
- ❌ **NEVER** include \`\`\` code fences in your response
- ❌ **NEVER** modify the code outside the boundaries

## Output Format

\`\`\`xml
<changes_made>
Brief description of what was changed
</changes_made>
<new_code>
[complete modified code without markers]
</new_code>
\`\`\`

## Example

### Input:
\`\`\`xml
<user_request>
INSTRUCTIONS:
Return 2 instead of 1

CODE:
import * as wmill from "windmill-client"

function test() {
	return "hello"
}

[#START]
export async function main() {
	return 1;
}
[#END]
</user_request>
\`\`\`

### Expected Output:
\`\`\`xml
<changes_made>
Changed return value from 1 to 2 in main function
</changes_made>
<new_code>
import * as wmill from "windmill-client"

function test() {
	return "hello"
}

export async function main() {
	return 2;
}
</new_code>
\`\`\`
`

const CHAT_USER_CODE_CONTEXT = `
- {title}:
\`\`\`{language}
{code}
\`\`\`
`

const CHAT_USER_ERROR_CONTEXT = `
ERROR:
{error}
`

export const CHAT_USER_PROMPT = `
INSTRUCTIONS:
{instructions}

WINDMILL LANGUAGE CONTEXT:
{lang_context}

`

export const CHAT_USER_DB_CONTEXT = `- {title}: SCHEMA: \n{schema}\n`

export function prepareScriptSystemMessage(): ChatCompletionSystemMessageParam {
	return {
		role: 'system',
		content: CHAT_SYSTEM_PROMPT
	}
}

const applyCodePieceToCodeContext = (codePieces: CodePieceElement[], codeContext: string) => {
	let code = codeContext.split('\n')
	let shiftOffset = 0
	codePieces.sort((a, b) => a.startLine - b.startLine)
	for (const codePiece of codePieces) {
		code.splice(codePiece.endLine + shiftOffset, 0, '[#END]')
		code.splice(codePiece.startLine + shiftOffset - 1, 0, '[#START]')
		shiftOffset += 2
	}
	return code.join('\n')
}

export function prepareScriptTools(
	language: ScriptLang | 'bunnative',
	context: ContextElement[]
): Tool<ScriptChatHelpers>[] {
	const tools: Tool<ScriptChatHelpers>[] = []
	if (['python3', 'php', 'bun', 'deno', 'nativets', 'bunnative'].includes(language)) {
		tools.push(resourceTypeTool)
	}
	if (context.some((c) => c.type === 'db')) {
		tools.push(dbSchemaTool)
	}
	if (['bun', 'deno'].includes(language)) {
		tools.push(createSearchHubScriptsTool(true))
		tools.push(searchNpmPackagesTool)
	}
	return tools
}

export async function prepareScriptUserMessage(
	instructions: string,
	language: ScriptLang | 'bunnative',
	selectedContext: ContextElement[],
	options: {
		isPreprocessor?: boolean
	} = {}
): Promise<ChatCompletionUserMessageParam> {
	let codeContext = 'CODE:\n'
	let errorContext = 'ERROR:\n'
	let dbContext = 'DATABASES:\n'
	let diffContext = 'DIFF:\n'
	let hasCode = false
	let hasError = false
	let hasDb = false
	let hasDiff = false
	for (const context of selectedContext) {
		if (context.type === 'code') {
			hasCode = true
			codeContext += CHAT_USER_CODE_CONTEXT.replace('{title}', context.title)
				.replace('{language}', scriptLangToEditorLang(language))
				.replace(
					'{code}',
					applyCodePieceToCodeContext(
						selectedContext.filter((c) => c.type === 'code_piece'),
						context.content
					)
				)
		} else if (context.type === 'error') {
			if (hasError) {
				throw new Error('Multiple error contexts provided')
			}
			hasError = true
			errorContext = CHAT_USER_ERROR_CONTEXT.replace('{error}', context.content)
		} else if (context.type === 'db') {
			hasDb = true
			dbContext += CHAT_USER_DB_CONTEXT.replace('{title}', context.title).replace(
				'{schema}',
				context.schema?.stringified ?? 'to fetch with get_db_schema'
			)
		} else if (context.type === 'diff') {
			hasDiff = true
			const diff = JSON.stringify(context.diff)
			diffContext = diff.length > 3000 ? diff.slice(0, 3000) + '...' : diff
		}
	}

	let userMessage = CHAT_USER_PROMPT.replace('{instructions}', instructions).replace(
		'{lang_context}',
		getLangContext(language, { allowResourcesFetch: true, ...options })
	)
	if (hasCode) {
		userMessage += codeContext
	}
	if (hasError) {
		userMessage += errorContext
	}
	if (hasDb) {
		userMessage += dbContext
	}
	if (hasDiff) {
		userMessage += diffContext
	}
	return {
		role: 'user',
		content: userMessage
	}
}

const RESOURCE_TYPE_FUNCTION_DEF: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'search_resource_types',
		description: 'Finds and returns resource types that are relevant to the specified query',
		parameters: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description:
						'The query to search for, e.g. specific integration (e.g. "stripe") or a specific feature (e.g. "send emails")'
				}
			},
			required: ['query'],
			additionalProperties: false
		},
		strict: true
	}
}

const DB_SCHEMA_FUNCTION_DEF: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'get_db_schema',
		description: 'Gets the schema of the database',
		parameters: {
			type: 'object',
			properties: {
				resourcePath: { type: 'string', description: 'The path of the database resource' }
			},
			required: ['resourcePath']
		}
	}
}

export const MAX_SCHEMA_LENGTH = 100000 * 3.5

async function formatDBSchema(dbSchema: DBSchema) {
	let { stringified } = dbSchema
	if (dbSchema.lang === 'graphql') {
		if (stringified.length > MAX_SCHEMA_LENGTH) {
			stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
		}
		return 'GRAPHQL SCHEMA:\n' + stringified
	} else {
		if (stringified.length > MAX_SCHEMA_LENGTH) {
			stringified = stringified.slice(0, MAX_SCHEMA_LENGTH) + '...'
		}
		return (
			'DATABASE SCHEMA (each column is in the format [name, type, required, default?]):\n' +
			stringified
		)
	}
}

export interface ScriptChatHelpers {
	getLang: () => ScriptLang | 'bunnative'
}

export const resourceTypeTool: Tool<ScriptChatHelpers> = {
	def: RESOURCE_TYPE_FUNCTION_DEF,
	fn: async ({ args, workspace, helpers, toolCallbacks, toolId }) => {
		toolCallbacks.setToolStatus(toolId, 'Searching resource types for "' + args.query + '"...')
		const formattedResourceTypes = await getFormattedResourceTypes(
			helpers.getLang(),
			args.query,
			workspace
		)
		toolCallbacks.setToolStatus(toolId, 'Retrieved resource types for "' + args.query + '"')
		return formattedResourceTypes
	}
}

export const dbSchemaTool: Tool<ScriptChatHelpers> = {
	def: DB_SCHEMA_FUNCTION_DEF,
	fn: async ({ args, workspace, toolCallbacks, toolId }) => {
		if (!args.resourcePath) {
			throw new Error('Database path not provided')
		}
		toolCallbacks.setToolStatus(toolId, 'Getting database schema for ' + args.resourcePath + '...')
		const resource = await ResourceService.getResource({
			workspace: workspace,
			path: args.resourcePath
		})
		const newDbSchemas = {}
		await getDbSchemas(
			resource.resource_type,
			args.resourcePath,
			workspace,
			newDbSchemas,
			(error) => {
				console.error(error)
			}
		)
		dbSchemas.update((schemas) => ({ ...schemas, ...newDbSchemas }))
		const dbs = get(dbSchemas)
		const db = dbs[args.resourcePath]
		if (!db) {
			throw new Error('Database not found')
		}
		const stringSchema = await formatDBSchema(db)
		toolCallbacks.setToolStatus(toolId, 'Retrieved database schema for ' + args.resourcePath)
		return stringSchema
	}
}

type PackageSearchQuery = {
	package: {
		name: string
		version: string
		links: {
			npm: string
			homepage: string
			repository: string
			bugs: string
		}
	}
	searchScore: number
}

type PackageSearchResult = {
	package: string
	documentation: string
	types: string
}

const packagesSearchCache = new Map<string, PackageSearchResult[]>()
export async function searchExternalIntegrationResources(args: { query: string }): Promise<string> {
	try {
		if (packagesSearchCache.has(args.query)) {
			return JSON.stringify(packagesSearchCache.get(args.query))
		}

		const result = await fetch(`https://registry.npmjs.org/-/v1/search?text=${args.query}&size=2`)
		const data = await result.json()
		const filtered = data.objects.filter(
			(r: PackageSearchQuery) => r.searchScore >= SCORE_THRESHOLD
		)

		const modelContextWindow = getModelContextWindow(get(copilotSessionModel)?.model ?? '')
		const results: PackageSearchResult[] = await Promise.all(
			filtered.map(async (r: PackageSearchQuery) => {
				let documentation = ''
				let types = ''
				try {
					const docResponse = await fetch(`https://unpkg.com/${r.package.name}/readme.md`)
					const docLimit = Math.floor((modelContextWindow * DOCS_CONTEXT_PERCENTAGE) / 100)
					documentation = await docResponse.text()
					documentation = documentation.slice(0, docLimit)
				} catch (error) {
					console.error('Error getting documentation for package:', error)
					documentation = ''
				}
				try {
					const typesResponse = await fetchNpmPackageTypes(r.package.name, r.package.version)
					const typesLimit = Math.floor((modelContextWindow * TYPES_CONTEXT_PERCENTAGE) / 100)
					types = typesResponse.types.slice(0, typesLimit)
				} catch (error) {
					console.error('Error getting types for package:', error)
					types = ''
				}
				return {
					package: r.package.name,
					documentation: documentation,
					types: types
				}
			})
		)
		packagesSearchCache.set(args.query, results)
		return JSON.stringify(results)
	} catch (error) {
		console.error('Error searching external integration resources:', error)
		return 'Error searching external integration resources'
	}
}

const SEARCH_NPM_PACKAGES_TOOL: ChatCompletionTool = {
	type: 'function',
	function: {
		name: 'search_npm_packages',
		description: 'Search for npm packages and their documentation',
		parameters: {
			type: 'object',
			properties: {
				query: {
					type: 'string',
					description: 'The query to search for'
				}
			},
			required: ['query']
		}
	}
}

export const searchNpmPackagesTool: Tool<ScriptChatHelpers> = {
	def: SEARCH_NPM_PACKAGES_TOOL,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, 'Searching for relevant packages...')
		const result = await searchExternalIntegrationResources(args)
		toolCallbacks.setToolStatus(toolId, 'Retrieved relevant packages')
		return result
	}
}

export async function fetchNpmPackageTypes(
	packageName: string,
	version: string = 'latest'
): Promise<{ success: boolean; types: string; error?: string }> {
	try {
		const typeDefinitions = new Map<string, string>()

		const ata = setupTypeAcquisition({
			projectName: 'NPM-Package-Types',
			depsParser: () => [],
			root: '',
			delegate: {
				receivedFile: (code: string, path: string) => {
					if (path.endsWith('.d.ts')) {
						typeDefinitions.set(path, code)
					}
				},
				localFile: () => {}
			}
		})

		const depsToGet: DepsToGet = [
			{
				raw: packageName,
				module: packageName,
				version: version
			}
		]

		await ata(depsToGet)

		if (typeDefinitions.size === 0) {
			return {
				success: false,
				types: '',
				error: `No type definitions found for ${packageName}`
			}
		}

		const formattedTypes = Array.from(typeDefinitions.entries())
			.map(([path, content]) => `// ${path}\n${content}`)
			.join('\n\n')

		return {
			success: true,
			types: formattedTypes
		}
	} catch (error) {
		console.error('Error fetching NPM package types:', error)
		return {
			success: false,
			types: '',
			error: `Error fetching package types: ${error instanceof Error ? error.message : 'Unknown error'}`
		}
	}
}
