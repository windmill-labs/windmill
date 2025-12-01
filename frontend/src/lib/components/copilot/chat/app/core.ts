import type {
	ChatCompletionSystemMessageParam,
	ChatCompletionUserMessageParam
} from 'openai/resources/chat/completions.mjs'
import { z } from 'zod'
import { createSearchHubScriptsTool, createToolDef, type Tool } from '../shared'
import { FlowService, ScriptService, type Flow, type Script } from '$lib/gen'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'

// Backend runnable types
export type BackendRunnableType = 'script' | 'flow' | 'hubscript' | 'inline'

export interface InlineScript {
	language: 'bun' | 'python3'
	content: string
}

export interface BackendRunnable {
	name: string
	type: BackendRunnableType
	staticInputs?: Record<string, any>
	// For inline scripts
	inlineScript?: InlineScript
	// For workspace scripts/flows and hub scripts
	path?: string
}

export interface AppFiles {
	frontend: Record<string, string>
	backend: Record<string, BackendRunnable>
}

export interface AppAIChatHelpers {
	// Frontend file operations
	getFrontendFiles: () => Record<string, string>
	setFrontendFile: (path: string, content: string) => void
	deleteFrontendFile: (path: string) => void
	// Backend runnable operations
	getBackendRunnables: () => Record<string, BackendRunnable>
	setBackendRunnable: (key: string, runnable: BackendRunnable) => void
	deleteBackendRunnable: (key: string) => void
	// Combined view
	getFiles: () => AppFiles
}

// ============= Frontend File Tools =============

const getFrontendFilesSchema = z.object({})
const getFrontendFilesToolDef = createToolDef(
	getFrontendFilesSchema,
	'get_frontend_files',
	'Get all frontend files in the raw app. Returns a record of file paths to their content.'
)

const setFrontendFileSchema = z.object({
	path: z.string().describe('The path of the frontend file to create or update (e.g., /index.tsx, /styles.css)'),
	content: z.string().describe('The content of the file')
})
const setFrontendFileToolDef = createToolDef(
	setFrontendFileSchema,
	'set_frontend_file',
	'Create or update a frontend file in the raw app'
)

const deleteFrontendFileSchema = z.object({
	path: z.string().describe('The path of the frontend file to delete')
})
const deleteFrontendFileToolDef = createToolDef(
	deleteFrontendFileSchema,
	'delete_frontend_file',
	'Delete a frontend file from the raw app'
)

// ============= Backend Runnable Tools =============

const getBackendRunnablesSchema = z.object({})
const getBackendRunnablesToolDef = createToolDef(
	getBackendRunnablesSchema,
	'get_backend_runnables',
	'Get all backend runnables in the raw app. Returns a record of runnable keys to their configuration.'
)

const inlineScriptSchema = z.object({
	language: z.enum(['bun', 'python3']).describe('The language of the inline script'),
	content: z.string().describe('The content of the inline script. Must have a main function as entrypoint.')
})

const setBackendRunnableSchema = z.object({
	key: z.string().describe('The unique key/identifier for the backend runnable (used to call it from frontend as backend.<key>())'),
	name: z.string().describe('A short summary/description of what the runnable does'),
	type: z.enum(['script', 'flow', 'hubscript', 'inline']).describe('The type of runnable: "inline" for custom code, "script" for workspace script, "flow" for workspace flow, "hubscript" for hub script'),
	staticInputs: z.record(z.any()).optional().describe('Static inputs that are not overridable by the frontend caller. Useful for workspace/hub scripts to pre-fill certain arguments.'),
	inlineScript: inlineScriptSchema.optional().describe('Required when type is "inline". Contains the language and content of the script.'),
	path: z.string().optional().describe('Required when type is "script", "flow", or "hubscript". The fully qualified path to the workspace script/flow or hub script.')
})
const setBackendRunnableToolDef = createToolDef(
	setBackendRunnableSchema,
	'set_backend_runnable',
	'Create or update a backend runnable. Use type "inline" for custom code, or reference existing workspace/hub scripts/flows.'
)

const deleteBackendRunnableSchema = z.object({
	key: z.string().describe('The key of the backend runnable to delete')
})
const deleteBackendRunnableToolDef = createToolDef(
	deleteBackendRunnableSchema,
	'delete_backend_runnable',
	'Delete a backend runnable from the raw app'
)

// ============= Combined Files Tool =============

const getFilesSchema = z.object({})
const getFilesToolDef = createToolDef(
	getFilesSchema,
	'get_files',
	'Get all files in the raw app, including both frontend files and backend runnables as separate records.'
)

// ============= Workspace Runnables Search =============

const listWorkspaceRunnablesSchema = z.object({
	query: z.string().describe('The search query to find workspace scripts and flows'),
	type: z.enum(['all', 'scripts', 'flows']).optional().describe('Filter by type: "scripts" for scripts only, "flows" for flows only, "all" for both. Defaults to "all".')
})
const listWorkspaceRunnablesToolDef = createToolDef(
	listWorkspaceRunnablesSchema,
	'list_workspace_runnables',
	'Search for workspace scripts and flows by query. Returns fully qualified paths that can be used in backend runnables.'
)

class WorkspaceRunnablesSearch {
	private uf: uFuzzy
	private workspace: string | undefined = undefined
	private scripts: Script[] | undefined = undefined
	private flows: Flow[] | undefined = undefined

	constructor() {
		this.uf = new uFuzzy()
	}

	private async initScripts(workspace: string) {
		if (this.scripts === undefined || this.workspace !== workspace) {
			this.scripts = await ScriptService.listScripts({ workspace })
			this.workspace = workspace
		}
	}

	private async initFlows(workspace: string) {
		if (this.flows === undefined || this.workspace !== workspace) {
			this.flows = await FlowService.listFlows({ workspace })
			this.workspace = workspace
		}
	}

	async searchScripts(query: string, workspace: string) {
		await this.initScripts(workspace)
		const scripts = this.scripts
		if (!scripts) return []

		const results = this.uf.search(
			scripts.map((s) => (emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')')),
			query.trim()
		)
		return results[2]?.map((id) => ({
			type: 'script' as const,
			path: scripts[id].path,
			summary: scripts[id].summary
		})) ?? []
	}

	async searchFlows(query: string, workspace: string) {
		await this.initFlows(workspace)
		const flows = this.flows
		if (!flows) return []

		const results = this.uf.search(
			flows.map((f) => (emptyString(f.summary) ? f.path : f.summary + ' (' + f.path + ')')),
			query.trim()
		)
		return results[2]?.map((id) => ({
			type: 'flow' as const,
			path: flows[id].path,
			summary: flows[id].summary
		})) ?? []
	}

	async search(query: string, workspace: string, type: 'all' | 'scripts' | 'flows' = 'all') {
		const results: { type: 'script' | 'flow'; path: string; summary: string }[] = []

		if (type === 'all' || type === 'scripts') {
			results.push(...await this.searchScripts(query, workspace))
		}
		if (type === 'all' || type === 'flows') {
			results.push(...await this.searchFlows(query, workspace))
		}

		return results
	}
}

const workspaceRunnablesSearch = new WorkspaceRunnablesSearch()

// ============= Tools Array =============

export const appTools: Tool<AppAIChatHelpers>[] = [
	// Combined files tool
	{
		def: getFilesToolDef,
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting all files...' })
			const files = helpers.getFiles()
			const frontendCount = Object.keys(files.frontend).length
			const backendCount = Object.keys(files.backend).length
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved ${frontendCount} frontend files and ${backendCount} backend runnables` })
			return JSON.stringify(files, null, 2)
		}
	},
	// Frontend tools
	{
		def: getFrontendFilesToolDef,
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting frontend files...' })
			const files = helpers.getFrontendFiles()
			const fileCount = Object.keys(files).length
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved ${fileCount} frontend files` })
			return JSON.stringify(files, null, 2)
		}
	},
	{
		def: setFrontendFileToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setFrontendFileSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Setting frontend file '${parsedArgs.path}'...` })
			helpers.setFrontendFile(parsedArgs.path, parsedArgs.content)
			toolCallbacks.setToolStatus(toolId, { content: `Frontend file '${parsedArgs.path}' updated` })
			return `Frontend file '${parsedArgs.path}' has been set successfully`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting frontend file...' })
		}
	},
	{
		def: deleteFrontendFileToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = deleteFrontendFileSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Deleting frontend file '${parsedArgs.path}'...` })
			helpers.deleteFrontendFile(parsedArgs.path)
			toolCallbacks.setToolStatus(toolId, { content: `Frontend file '${parsedArgs.path}' deleted` })
			return `Frontend file '${parsedArgs.path}' has been deleted successfully`
		}
	},
	// Backend tools
	{
		def: getBackendRunnablesToolDef,
		fn: async ({ helpers, toolId, toolCallbacks }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Getting backend runnables...' })
			const runnables = helpers.getBackendRunnables()
			const count = Object.keys(runnables).length
			toolCallbacks.setToolStatus(toolId, { content: `Retrieved ${count} backend runnables` })
			return JSON.stringify(runnables, null, 2)
		}
	},
	{
		def: setBackendRunnableToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = setBackendRunnableSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Setting backend runnable '${parsedArgs.key}'...` })

			const runnable: BackendRunnable = {
				name: parsedArgs.name,
				type: parsedArgs.type,
				...(parsedArgs.staticInputs && { staticInputs: parsedArgs.staticInputs }),
				...(parsedArgs.inlineScript && { inlineScript: parsedArgs.inlineScript }),
				...(parsedArgs.path && { path: parsedArgs.path })
			}

			helpers.setBackendRunnable(parsedArgs.key, runnable)
			toolCallbacks.setToolStatus(toolId, { content: `Backend runnable '${parsedArgs.key}' updated` })
			return `Backend runnable '${parsedArgs.key}' has been set successfully`
		},
		preAction: ({ toolCallbacks, toolId }) => {
			toolCallbacks.setToolStatus(toolId, { content: 'Setting backend runnable...' })
		}
	},
	{
		def: deleteBackendRunnableToolDef,
		fn: async ({ args, helpers, toolId, toolCallbacks }) => {
			const parsedArgs = deleteBackendRunnableSchema.parse(args)
			toolCallbacks.setToolStatus(toolId, { content: `Deleting backend runnable '${parsedArgs.key}'...` })
			helpers.deleteBackendRunnable(parsedArgs.key)
			toolCallbacks.setToolStatus(toolId, { content: `Backend runnable '${parsedArgs.key}' deleted` })
			return `Backend runnable '${parsedArgs.key}' has been deleted successfully`
		}
	},
	// Search tools
	{
		def: listWorkspaceRunnablesToolDef,
		fn: async ({ args, workspace, toolId, toolCallbacks }) => {
			const parsedArgs = listWorkspaceRunnablesSchema.parse(args)
			const type = parsedArgs.type ?? 'all'
			toolCallbacks.setToolStatus(toolId, { content: `Searching workspace ${type} for "${parsedArgs.query}"...` })

			const results = await workspaceRunnablesSearch.search(parsedArgs.query, workspace, type)

			toolCallbacks.setToolStatus(toolId, { content: `Found ${results.length} workspace runnables matching "${parsedArgs.query}"` })
			return JSON.stringify(results, null, 2)
		}
	},
	// Hub scripts search (reuse from shared)
	createSearchHubScriptsTool(false)
]

export function prepareAppSystemMessage(customPrompt?: string): ChatCompletionSystemMessageParam {
	let content = `You are a helpful assistant that creates and edits apps on the Windmill platform. Apps are defined as a collection of files that contains both the frontend and the backend.

## App Structure

### Frontend
- The frontend is bundled using esbuild with entrypoint \`index.tsx\`
- Frontend files are managed separately from backend runnables
- The \`wmill.d.ts\` file is generated automatically from the backend runnables shape

### Backend
Backend runnables can be of different types:
- **inline**: Custom code written directly in the app (TypeScript/Bun or Python)
- **script**: Reference to a workspace script by path
- **flow**: Reference to a workspace flow by path
- **hubscript**: Reference to a hub script by path

Frontend calls backend using \`await backend.<runnable_key>(args...)\`.

For inline scripts, the code must have a \`main\` function as its entrypoint.

## Available Tools

### File Management
- \`get_files()\`: Get both frontend files and backend runnables
- \`get_frontend_files()\`: Get all frontend files
- \`set_frontend_file(path, content)\`: Create or update a frontend file
- \`delete_frontend_file(path)\`: Delete a frontend file
- \`get_backend_runnables()\`: Get all backend runnables
- \`set_backend_runnable(key, name, type, ...)\`: Create or update a backend runnable
- \`delete_backend_runnable(key)\`: Delete a backend runnable

### Discovery
- \`list_workspace_runnables(query, type?)\`: Search workspace scripts and flows
- \`search_hub_scripts(query)\`: Search hub scripts

## Backend Runnable Configuration

When creating a backend runnable with \`set_backend_runnable\`:

1. **For inline scripts** (type: "inline"):
   \`\`\`
   {
     key: "myFunction",
     name: "Does something useful",
     type: "inline",
     inlineScript: {
       language: "bun",  // or "python3"
       content: "export async function main(arg1: string) { return result; }"
     }
   }
   \`\`\`

2. **For workspace scripts** (type: "script"):
   \`\`\`
   {
     key: "sendEmail",
     name: "Send email via SMTP",
     type: "script",
     path: "f/folder/send_email",
     staticInputs: { smtp_server: "mail.example.com" }  // optional pre-filled inputs
   }
   \`\`\`

3. **For workspace flows** (type: "flow"):
   \`\`\`
   {
     key: "processOrder",
     name: "Process customer order",
     type: "flow",
     path: "f/folder/process_order_flow"
   }
   \`\`\`

4. **For hub scripts** (type: "hubscript"):
   \`\`\`
   {
     key: "slackMessage",
     name: "Send Slack message",
     type: "hubscript",
     path: "hub/123/slack/send_message"
   }
   \`\`\`

## Instructions

Follow the user instructions carefully. When creating a new app:
1. First use \`get_files\` to see the current state
2. Create frontend files using \`set_frontend_file\`
3. Create backend runnables using \`set_backend_runnable\`
4. Use \`list_workspace_runnables\` or \`search_hub_scripts\` to find existing scripts/flows to reuse

Explain what you're doing as you work. Show file contents before setting them when making significant changes.
`

	if (customPrompt?.trim()) {
		content = `${content}\n\nUSER GIVEN INSTRUCTIONS:\n${customPrompt.trim()}`
	}

	return {
		role: 'system',
		content
	}
}

export function prepareAppUserMessage(
	instructions: string,
	files?: AppFiles
): ChatCompletionUserMessageParam {
	let content = ''

	if (files) {
		if (Object.keys(files.frontend).length > 0) {
			content += `## CURRENT FRONTEND FILES:\n`
			for (const [path, fileContent] of Object.entries(files.frontend)) {
				content += `\n### ${path}\n\`\`\`\n${fileContent}\n\`\`\`\n`
			}
			content += '\n'
		}

		if (Object.keys(files.backend).length > 0) {
			content += `## CURRENT BACKEND RUNNABLES:\n`
			content += '\`\`\`json\n' + JSON.stringify(files.backend, null, 2) + '\n\`\`\`\n\n'
		}
	}

	content += `## INSTRUCTIONS:\n${instructions}`

	return {
		role: 'user',
		content
	}
}
