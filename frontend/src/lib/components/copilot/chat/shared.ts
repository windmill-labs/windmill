import type {
	ChatCompletionFunctionTool,
	ChatCompletionMessageFunctionToolCall,
	ChatCompletionMessageParam
} from 'openai/resources/chat/completions.mjs'
import type { UserDraftItemKind } from '$lib/gen'

/**
 * Special module IDs used throughout the flow system
 */
export const SPECIAL_MODULE_IDS = {
	/** The flow input schema node */
	INPUT: 'Input',
	/** The preprocessor module that runs before the flow */
	PREPROCESSOR: 'preprocessor',
	/** The failure handler module */
	FAILURE: 'failure'
} as const
import { get } from 'svelte/store'
import type { PasteAttachment } from './pasteTokens'
import type { CodePieceElement, ContextElement, FlowModuleCodePieceElement } from './context'
import { workspaceStore } from '$lib/stores'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import { findModuleInFlow, findModuleInModules } from '$lib/components/flows/flowTree'
import type { FunctionParameters } from 'openai/resources/shared.mjs'
import { z } from 'zod'
import {
	ScriptService,
	FlowService,
	JobService,
	type Job,
	type CompletedJob,
	type FlowValue,
	type FlowModule,
	type ScriptLang,
	type Script,
	type Flow
} from '$lib/gen'
import uFuzzy from '@leeoniya/ufuzzy'
import { emptyString } from '$lib/utils'
import { forLater } from '$lib/forLater'
import { scriptLangToEditorLang } from '$lib/scripts'
import { getCurrentModel } from '$lib/aiStore'
import { type editor as meditor } from 'monaco-editor'

// Prettify function for code arguments - extracts and formats code from JSON
function prettifyCodeArguments(content: string): string {
	let codeContent = content

	// If it's a JSON string, try to extract the code property
	if (typeof content === 'string' && content.trim().startsWith('{')) {
		try {
			const parsed = JSON.parse(content)
			if (parsed.code) {
				codeContent = parsed.code
			}
		} catch {
			// If JSON is incomplete during streaming, try to extract manually
			// Remove leading { "code": " or {"code":"
			codeContent = content.replace(/^\{\s*"code"\s*:\s*"/, '')
			// Remove trailing } if it exists
			codeContent = codeContent.replace(/"\s*}\s*$/, '')
		}
	}

	// Convert escaped newlines to actual newlines
	codeContent = codeContent.replace(/\\n/g, '\n')

	// Convert other common escape sequences
	codeContent = codeContent.replace(/\\t/g, '\t')
	codeContent = codeContent.replace(/\\"/g, '"')
	codeContent = codeContent.replace(/\\\\/g, '\\')

	return codeContent
}

function decodeEscapedToolString(content: string): string {
	return content
		.replace(/\\n/g, '\n')
		.replace(/\\t/g, '\t')
		.replace(/\\"/g, '"')
		.replace(/\\\\/g, '\\')
}

function extractJsonStringProperty(content: string, property: string): string | undefined {
	const propertyKey = `"${property}"`
	const propertyIndex = content.indexOf(propertyKey)
	if (propertyIndex === -1) {
		return undefined
	}

	let cursor = propertyIndex + propertyKey.length
	while (cursor < content.length && /\s/.test(content[cursor] ?? '')) {
		cursor++
	}
	if (content[cursor] !== ':') {
		return undefined
	}

	cursor++
	while (cursor < content.length && /\s/.test(content[cursor] ?? '')) {
		cursor++
	}
	if (content[cursor] !== '"') {
		return undefined
	}

	const start = cursor + 1
	let escaped = false
	for (let index = start; index < content.length; index++) {
		const char = content[index]
		if (escaped) {
			escaped = false
			continue
		}
		if (char === '\\') {
			escaped = true
			continue
		}
		if (char === '"') {
			return content.slice(start, index)
		}
	}

	return content.slice(start)
}

// Prettify function for set_module_code - extracts code from moduleId/code JSON
function prettifySetModuleCode(content: string): string {
	let codeContent = content

	if (typeof content === 'string' && content.trim().startsWith('{')) {
		try {
			const parsed = JSON.parse(content)
			if (parsed.code) {
				codeContent = parsed.code
			}
		} catch {
			const extractedCode = extractJsonStringProperty(content, 'code')
			if (extractedCode !== undefined) {
				codeContent = extractedCode
			}
		}
	}

	return decodeEscapedToolString(codeContent)
}

function prettifyPatchReplacement(content: string): string {
	let newString: string | undefined

	if (typeof content === 'string' && content.trim().startsWith('{')) {
		try {
			const parsed = JSON.parse(content)
			if (typeof parsed.new_string === 'string') {
				newString = parsed.new_string
			}
		} catch {
			newString = extractJsonStringProperty(content, 'new_string')
		}
	}

	if (newString === undefined) {
		return content
	}

	return decodeEscapedToolString(newString)
}

function prettifyPatchFlowJson(content: string): string {
	return prettifyPatchReplacement(content)
}

function prettifyPatchFile(content: string): string {
	return prettifyPatchReplacement(content)
}

// Prettify function for module value JSON - extracts the 'value' property and formats it
function prettifyModuleValue(content: string): string {
	try {
		const parsed = JSON.parse(content)
		// Extract just the 'value' property (the actual module definition)
		if (parsed.value) {
			return JSON.stringify(parsed.value, null, 2)
		}
		return JSON.stringify(parsed, null, 2)
	} catch {
		// If JSON is incomplete during streaming, try to extract the value property manually
		const valueMatch = content.match(/"value"\s*:\s*(\{[\s\S]*)$/)
		if (valueMatch) {
			let valueContent = valueMatch[1]
			// Try to parse and format the extracted value
			try {
				// Find the matching closing brace for the value object
				let braceCount = 0
				let endIndex = 0
				for (let i = 0; i < valueContent.length; i++) {
					if (valueContent[i] === '{') braceCount++
					else if (valueContent[i] === '}') braceCount--
					if (braceCount === 0) {
						endIndex = i + 1
						break
					}
				}
				if (endIndex > 0) {
					const valueJson = valueContent.substring(0, endIndex)
					const parsed = JSON.parse(valueJson)
					return JSON.stringify(parsed, null, 2)
				}
			} catch {
				// If parsing fails, just unescape and return the extracted value content
				valueContent = valueContent.replace(/\\n/g, '\n')
				valueContent = valueContent.replace(/\\t/g, '\t')
				valueContent = valueContent.replace(/\\"/g, '"')
				valueContent = valueContent.replace(/\\\\/g, '\\')
				return valueContent
			}
		}
		// Fallback: just unescape and return
		let result = content
		result = result.replace(/\\n/g, '\n')
		result = result.replace(/\\t/g, '\t')
		result = result.replace(/\\"/g, '"')
		result = result.replace(/\\\\/g, '\\')
		return result
	}
}

// Map of tool names to their prettify functions
export const TOOL_PRETTIFY_MAP: Record<string, (content: string) => string> = {
	edit_code: prettifyCodeArguments,
	set_module_code: prettifySetModuleCode,
	patch_flow_json: prettifyPatchFlowJson,
	patch_file: prettifyPatchFile,
	add_module: prettifyModuleValue,
	modify_module: prettifyModuleValue
}

export interface ContextStringResult {
	dbContext: string
	diffContext: string
	flowModuleContext: string
	hasDb: boolean
	hasDiff: boolean
	hasFlowModule: boolean
}

/** Count exact occurrences of `search` in `content`. */
export function countExactMatches(content: string, search: string): number {
	if (search.length === 0) return 0
	let count = 0
	let index = 0
	while ((index = content.indexOf(search, index)) !== -1) {
		count++
		index += search.length
	}
	return count
}

/**
 * Replace exact occurrences of `oldString` with `newString` in `content`.
 * When `replaceAll` is false, only the first match is replaced. Returns the
 * original string unchanged when no match is found.
 */
export function applyExactReplace(
	content: string,
	oldString: string,
	newString: string,
	replaceAll: boolean
): string {
	if (replaceAll) return content.split(oldString).join(newString)
	const index = content.indexOf(oldString)
	if (index === -1) return content
	return content.slice(0, index) + newString + content.slice(index + oldString.length)
}

/**
 * Match-count-validated exact text replacement. Throws when `oldString` is
 * missing, and (unless `replaceAll`) when it appears more than once.
 * `contextLabel` flows into the error message ("not found in the <label>.").
 */
export function findAndReplace(
	content: string,
	oldString: string,
	newString: string,
	replaceAll: boolean,
	contextLabel: string
): string {
	const matchCount = countExactMatches(content, oldString)
	if (matchCount === 0) {
		throw new Error(`old_string was not found in the ${contextLabel}.`)
	}
	if (!replaceAll && matchCount !== 1) {
		throw new Error(
			`old_string matched ${matchCount} locations. Make it more specific or set replace_all to true.`
		)
	}
	return applyExactReplace(content, oldString, newString, replaceAll)
}

export const extractAllModules = (modules: FlowModule[]): FlowModule[] => {
	return modules.flatMap((m) => {
		if (m.value.type === 'forloopflow' || m.value.type === 'whileloopflow') {
			return [m, ...extractAllModules(m.value.modules)]
		}
		if (m.value.type === 'branchall') {
			return [m, ...extractAllModules(m.value.branches.flatMap((b) => b.modules))]
		}
		if (m.value.type === 'branchone') {
			return [
				m,
				...extractAllModules([...m.value.branches.flatMap((b) => b.modules), ...m.value.default])
			]
		}
		return [m]
	})
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

export function applyCodePiecesToFlowModules(
	codePieces: FlowModuleCodePieceElement[],
	flowModules: FlowModule[]
): FlowModule[] {
	const moduleCodePieces = new Map<string, FlowModuleCodePieceElement[]>()
	for (const codePiece of codePieces) {
		const moduleId = codePiece.id
		if (!moduleCodePieces.has(moduleId)) {
			moduleCodePieces.set(moduleId, [])
		}
		moduleCodePieces.get(moduleId)!.push(codePiece)
	}

	// Clone modules to avoid mutation
	const modifiedModules = JSON.parse(JSON.stringify(flowModules))

	// Apply code pieces to each module
	for (const [moduleId, pieces] of moduleCodePieces) {
		const module = findModuleInModules(modifiedModules, moduleId)
		if (module && module.value.type === 'rawscript' && module.value.content) {
			module.value.content = applyCodePieceToCodeContext(
				pieces as unknown as CodePieceElement[],
				module.value.content
			)
		}
	}

	return modifiedModules
}

export function buildContextString(selectedContext: ContextElement[]): string {
	const dbTemplate = `- {title}: SCHEMA: \n{schema}\n`
	const codeTemplate = `
	- {title}:
	\`\`\`{language}
	{code}
	\`\`\`
	`

	let dbContext = 'DATABASES:\n'
	let diffContext = 'DIFF:\n'
	let flowModuleContext = 'FOCUSED FLOW MODULES IDS:\n'
	let codeContext = 'CODE:\n'
	let errorContext = `
	ERROR:
	{error}
	`
	let hasCode = false
	let hasDb = false
	let hasDiff = false
	let hasFlowModule = false
	let hasError = false
	let workspaceItemsContext = ''

	let result = '\n\n'
	for (const context of selectedContext) {
		if (context.type === 'code') {
			hasCode = true
			codeContext += codeTemplate
				.replace('{title}', context.title)
				.replace('{language}', scriptLangToEditorLang(context.lang))
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
			errorContext = errorContext.replace('{error}', context.content)
		} else if (context.type === 'db') {
			hasDb = true
			dbContext += dbTemplate
				.replace('{title}', context.title)
				.replace('{schema}', context.schema?.stringified ?? 'to fetch with get_db_schema')
			dbContext += '\n'
		} else if (context.type === 'diff') {
			hasDiff = true
			const diff = JSON.stringify(context.diff)
			diffContext += (diff.length > 3000 ? diff.slice(0, 3000) + '...' : diff) + '\n'
		} else if (context.type === 'flow_module') {
			hasFlowModule = true
			flowModuleContext += `${context.id}\n`
		} else if (context.type === 'workspace_script') {
			if (!workspaceItemsContext) {
				workspaceItemsContext = 'SELECTED WORKSPACE ITEMS:\n'
			}
			workspaceItemsContext += `- type: script, path: ${context.path}\n`
		} else if (context.type === 'workspace_flow') {
			if (!workspaceItemsContext) {
				workspaceItemsContext = 'SELECTED WORKSPACE ITEMS:\n'
			}
			workspaceItemsContext += `- type: flow, path: ${context.path}\n`
		} else if (context.type === 'workspace_app') {
			if (!workspaceItemsContext) {
				workspaceItemsContext = 'SELECTED WORKSPACE ITEMS:\n'
			}
			workspaceItemsContext += `- type: raw_app, path: ${context.path}\n`
		}
	}

	if (hasCode) {
		result += '\n' + codeContext
	}
	if (hasError) {
		result += '\n' + errorContext
	}
	if (hasDb) {
		result += '\n' + dbContext
	}
	if (hasDiff) {
		result += '\n' + diffContext
	}
	if (hasFlowModule) {
		result += '\n' + flowModuleContext
	}
	if (workspaceItemsContext) {
		result += '\n' + workspaceItemsContext
	}

	return result
}

type BaseDisplayMessage = {
	content: string
	contextElements?: ContextElement[]
	snapshot?: { type: 'flow'; value: ExtendedOpenFlow } | { type: 'app'; value: number }
}

export type UserDisplayMessage = BaseDisplayMessage & {
	role: 'user'
	index: number // Used to match index with actual chat messages
	error?: boolean
	// Collapsed big-paste blobs referenced by tokens in `content`. Lets the
	// bubble render/expand chips; the LLM message stores the expanded text.
	pastes?: PasteAttachment[]
}

export type CreatedResourceTriggerKind =
	| 'http'
	| 'websocket'
	| 'kafka'
	| 'nats'
	| 'postgres'
	| 'mqtt'
	| 'sqs'
	| 'gcp'
	| 'azure'
	| 'email'

export type CreatedResourceAction = {
	id: string
	type: 'open_created_resource'
	label: string
	resource: 'schedule' | 'trigger' | 'resource' | 'variable'
	path: string
	targetKind?: 'script' | 'flow'
	triggerKind?: CreatedResourceTriggerKind
}

export type ToolDisplayAction = CreatedResourceAction

export type UserQuestionDisplay = {
	question: string
	choices: string[]
	selectedChoice?: string
	canceled?: boolean
}

export type ToolDisplayMessage = {
	role: 'tool'
	tool_call_id: string
	content: string
	parameters?: any
	result?: any
	logs?: string
	isLoading?: boolean
	error?: string
	needsConfirmation?: boolean
	showDetails?: boolean
	autoCollapseDetails?: boolean
	isStreamingArguments?: boolean
	toolName?: string
	showFade?: boolean
	actions?: ToolDisplayAction[]
	userQuestion?: UserQuestionDisplay
}

export type AssistantDisplayMessage = BaseDisplayMessage & {
	role: 'assistant'
	/** Summarized reasoning/thinking text streamed before the answer (Anthropic + compat providers). */
	reasoning?: string
	/**
	 * True only on the synthetic live message appended while tokens stream
	 * (see AIChat.svelte). Finalized messages never set it — without the flag,
	 * a reasoning-only message (thinking that led straight to a tool call)
	 * would look like it is still streaming forever.
	 */
	streaming?: boolean
}

/**
 * Compaction boundary: replaces the summarized prefix in BOTH displayMessages
 * and the API messages (where it is a plain user message). It carries no index
 * because it is never a restart target — only the surviving tail's user
 * messages are rewound to.
 */
export type SummaryDisplayMessage = {
	role: 'summary'
	content: string
}

export type DisplayMessage =
	| UserDisplayMessage
	| ToolDisplayMessage
	| AssistantDisplayMessage
	| SummaryDisplayMessage

// A tool message whose askUserQuestion is still awaiting an answer: the AI loop
// is paused on the user. Drives the question card's interactivity, the
// "waiting for user" indicator, and disabling the main chat input — keep those
// in sync by going through this single predicate.
export function isActiveUserQuestion(message: DisplayMessage | undefined): boolean {
	return Boolean(
		message &&
			message.role === 'tool' &&
			message.userQuestion &&
			message.isLoading &&
			!message.error &&
			!message.userQuestion.selectedChoice &&
			!message.userQuestion.canceled
	)
}

// Fires after every tool call resolves, with the tool name. Lets a host (e.g.
// the sessions page) react to mutating tools — refreshing previews — without
// the tool layer knowing about the UI. Single slot; the consumer filters by name
// and reads the tool args (e.g. the mutated item's `path`) to scope its refresh.
let toolCompletionListener: ((toolName: string, args: any) => void) | undefined

export function setToolCompletionListener(
	fn: ((toolName: string, args: any) => void) | undefined
): void {
	toolCompletionListener = fn
}

async function callTool<T>({
	tools,
	functionName,
	args,
	workspace,
	helpers,
	toolCallbacks,
	toolId
}: {
	tools: Tool<T>[]
	functionName: string
	args: any
	workspace: string
	helpers: T
	toolCallbacks: ToolCallbacks
	toolId: string
}): Promise<string> {
	const tool = tools.find((t) => t.def.function.name === functionName)
	if (!tool) {
		throw new Error(
			`Unknown tool call: ${functionName}. Probably not in the correct mode, use the change_mode tool to switch to the correct mode.`
		)
	}
	const result = await tool.fn({ args, workspace, helpers, toolCallbacks, toolId })
	toolCompletionListener?.(functionName, args)
	return result
}

type MaybePromise<T> = T | Promise<T>

export async function processToolCall<T>({
	tools,
	toolCall,
	helpers,
	toolCallbacks,
	workspace
}: {
	tools: Tool<T>[]
	toolCall: ChatCompletionMessageFunctionToolCall
	helpers: T
	toolCallbacks: ToolCallbacks
	workspace?: string
}): Promise<ChatCompletionMessageParam> {
	try {
		const args = JSON.parse(toolCall.function.arguments || '{}')
		const tool = tools.find((t) => t.def.function.name === toolCall.function.name)
		const workspaceId = workspace ?? get(workspaceStore) ?? ''

		const validationError = await tool?.validateBeforeConfirmation?.({
			args,
			workspace: workspaceId,
			helpers
		})
		if (validationError) {
			toolCallbacks.setToolStatus(toolCall.id, {
				content: validationError,
				parameters: args,
				isLoading: false,
				isStreamingArguments: false,
				error: validationError,
				needsConfirmation: false,
				showDetails: tool?.showDetails,
				autoCollapseDetails: tool?.autoCollapseDetails
			})
			return {
				role: 'tool' as const,
				tool_call_id: toolCall.id,
				content: validationError
			}
		}

		// Check if tool requires confirmation
		const requiresConfirmation = tool?.requiresConfirmation === true
		const autoAcceptConfirmation =
			requiresConfirmation && toolCallbacks.shouldAutoAcceptToolConfirmations?.() === true
		const needsConfirmation = requiresConfirmation && !autoAcceptConfirmation

		toolCallbacks.setToolStatus(toolCall.id, {
			...(requiresConfirmation
				? { content: tool.confirmationMessage ?? 'Waiting for confirmation...' }
				: {}),
			parameters: args,
			isLoading: true,
			needsConfirmation: needsConfirmation,
			showDetails: tool?.showDetails,
			autoCollapseDetails: tool?.autoCollapseDetails
		})

		// If confirmation is needed and we have the callback, wait for it
		if (needsConfirmation && toolCallbacks.requestConfirmation) {
			const confirmed = await toolCallbacks.requestConfirmation(toolCall.id)

			if (!confirmed) {
				toolCallbacks.setToolStatus(toolCall.id, {
					content: 'Cancelled by user',
					isLoading: false,
					isStreamingArguments: false,
					error: 'Tool execution was cancelled by user',
					needsConfirmation: false
				})
				return {
					role: 'tool' as const,
					tool_call_id: toolCall.id,
					content: 'Tool execution was cancelled by user'
				}
			}

			// Update status to executing after confirmation
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: true,
				needsConfirmation: false
			})
		}

		let result = ''
		try {
			result = await callTool({
				tools,
				functionName: toolCall.function.name,
				args,
				workspace: workspaceId,
				helpers,
				toolCallbacks,
				toolId: toolCall.id
			})
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false,
				isStreamingArguments: false
			})
		} catch (err) {
			console.error(err)
			toolCallbacks.setToolStatus(toolCall.id, {
				isLoading: false,
				isStreamingArguments: false,
				error: 'An error occurred while calling the tool'
			})
			const errorMessage =
				typeof err === 'object' && 'message' in err
					? err.message
					: typeof err === 'string'
						? err
						: 'An error occurred while calling the tool'
			result = `Error while calling tool: ${errorMessage}`
		}
		const toAdd = {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content: result
		}
		return toAdd
	} catch (err) {
		console.error(err)
		return {
			role: 'tool' as const,
			tool_call_id: toolCall.id,
			content: 'Error while calling tool'
		}
	}
}

export interface Tool<T> {
	def: ChatCompletionFunctionTool
	fn: (p: {
		args: any
		workspace: string
		helpers: T
		toolCallbacks: ToolCallbacks
		toolId: string
	}) => Promise<string>
	preAction?: (p: { toolCallbacks: ToolCallbacks; toolId: string }) => void
	validateBeforeConfirmation?: (p: {
		args: any
		workspace: string
		helpers: T
	}) => MaybePromise<string | undefined>
	setSchema?: (helpers: any) => Promise<void>
	requiresConfirmation?: boolean
	confirmationMessage?: string
	showDetails?: boolean
	autoCollapseDetails?: boolean
	streamArguments?: boolean
	showFade?: boolean
}

/** Status of a job the chat started and tracks in the jobs tray. Mirrors the
 * runs page: `suspended` = a flow step waiting for approval, `scheduled` = a run
 * scheduled for later. Kept in lockstep with `ChatJob.job` (see below). */
export type ChatJobStatus =
	| 'queued'
	| 'running'
	| 'suspended'
	| 'scheduled'
	| 'success'
	| 'failure'
	| 'canceled'

/** A job the chat started and is tracking. Rendered in the jobs tray, persisted
 * with the chat, and advanced by a single background poller on the manager. */
export type ChatJob = {
	jobId: string
	/** Pairs with the ToolDisplayMessage card that launched it. */
	toolCallId: string
	kind: 'script' | 'flow'
	/** Path or step label shown in the tray row. */
	label: string
	workspace: string
	createdAt: number
	status: ChatJobStatus
	durationMs?: number
	/** True once it left the inline wait and is polled in the background. */
	detached: boolean
	/** Notify-only: whether its completion has been surfaced to the model yet. */
	reported: boolean
	/** Trimmed snapshot of the last fetched Job (heavy fields stripped, see
	 * `trimJob`), fed to `<JobStatusIcon>` so the tray badge matches the runs page
	 * exactly. Always written together with `status` from the SAME job so the two
	 * can't drift. Undefined only before the first fetch. */
	job?: Job
}

/** Derive the tray status from a fetched Job. Deliberately mirrors the branch
 * order of JobStatusIcon.svelte so the scalar status and the badge never
 * disagree. */
export function deriveChatJobStatus(job: Job): ChatJobStatus {
	if ('success' in job) {
		return job.canceled ? 'canceled' : job.success ? 'success' : 'failure'
	}
	// QueuedJob
	if (job.running && job.suspend) return 'suspended'
	if (job.running) return 'running'
	if (job.scheduled_for && forLater(job.scheduled_for)) return 'scheduled'
	return 'queued'
}

/** Strip the heavy fields from a fetched Job before storing it on a ChatJob (the
 * tray only needs the status-discriminant scalars JobStatusIcon reads).
 *
 * MUST clone + delete — never rebuild as an object literal. JobStatusIcon
 * discriminates with the `in` operator (`'success' in job`), which tests KEY
 * PRESENCE, not truthiness. A literal that always carries a `success` key would
 * make every running/queued job misrender as a completed (failed) job. */
export function trimJob(job: Job): Job {
	const trimmed = { ...job } as Record<string, unknown>
	delete trimmed.logs
	delete trimmed.args
	delete trimmed.result
	delete trimmed.raw_code
	delete trimmed.raw_flow
	delete trimmed.flow_status
	return trimmed as unknown as Job
}

/** The subset supplied when a job first starts; the manager fills in the rest. */
export type ChatJobInit = Pick<ChatJob, 'jobId' | 'toolCallId' | 'kind' | 'label' | 'workspace'>

export interface ToolCallbacks {
	setToolStatus: (id: string, metadata?: Partial<ToolDisplayMessage>) => void
	removeToolStatus: (id: string) => void
	/** Job-tracking hooks, wired only by the global/sessions chat (mode === GLOBAL).
	 * Their presence is what enables detach-into-background in executeTestRun; when
	 * absent (in-editor script/flow/pipeline chats), test runs keep blocking the
	 * loop with the legacy 60s cap. */
	onJobStarted?: (job: ChatJobInit) => void
	onJobStatus?: (jobId: string, update: Partial<ChatJob>) => void
	onJobDetached?: (jobId: string) => void
	/** Streamed reasoning/thinking deltas, rendered as a collapsible block in the chat. */
	onReasoningDelta?: (token: string) => void
	/** Fired when the model starts reasoning — drives a "Thinking" indicator even when
	 * no summary text is returned (e.g. OpenAI reasoning models). */
	onReasoningStart?: () => void
	requestConfirmation?: (toolId: string) => Promise<boolean>
	shouldAutoAcceptToolConfirmations?: () => boolean
	requestUserQuestion?: (
		toolId: string,
		question: UserQuestionDisplay
	) => Promise<string | undefined>
	/** Records a workspace item the tool call created/edited/deleted, by its
	 * canonical (itemKind, storagePath). Session chats wire this to accumulate the
	 * chat's modified-items mask; the global side-panel chat omits it (no-op). */
	onItemModified?: (itemKind: UserDraftItemKind, storagePath: string) => void
	/** A tool deployed a draft: the mask entry moves from the draft's storage path
	 * to the deployed path (they differ for synthetic draft-only storage keys). */
	onItemDeployed?: (itemKind: UserDraftItemKind, storagePath: string, deployedPath: string) => void
	/** A tool discarded a draft: the chat's touch on the item is undone. */
	onItemDiscarded?: (itemKind: UserDraftItemKind, storagePath: string) => void
}

export function createToolDef(
	zodSchema: z.ZodSchema,
	name: string,
	description: string,
	{ strict = true }: { strict?: boolean } = {} // we sometimes have to set strict to false for open ai models to avoid issues with complex properties
): ChatCompletionFunctionTool {
	// console.log('creating tool def for', name, zodSchema)
	let parameters = z.toJSONSchema(zodSchema)
	delete parameters.$schema
	if (!parameters.required) parameters.required = []
	normalizeToolParameterSchema(parameters)
	const effectiveStrict = strict && !hasOptionalProperties(parameters)

	return {
		type: 'function',
		function: {
			strict: effectiveStrict,
			name,
			description,
			parameters
		}
	}
}

function hasOptionalProperties(schema: Record<string, any> | undefined): boolean {
	if (!schema || typeof schema !== 'object') {
		return false
	}

	if (schema.properties && typeof schema.properties === 'object') {
		const required = new Set(Array.isArray(schema.required) ? schema.required : [])
		const propertyKeys = Object.keys(schema.properties)
		if (propertyKeys.some((key) => !required.has(key))) {
			return true
		}
		for (const key of propertyKeys) {
			if (hasOptionalProperties(schema.properties[key])) {
				return true
			}
		}
	}

	if (schema.items) {
		if (Array.isArray(schema.items)) {
			if (schema.items.some((item) => hasOptionalProperties(item))) {
				return true
			}
		} else if (hasOptionalProperties(schema.items)) {
			return true
		}
	}

	if (
		schema.additionalProperties &&
		typeof schema.additionalProperties === 'object' &&
		hasOptionalProperties(schema.additionalProperties)
	) {
		return true
	}

	for (const key of ['allOf', 'anyOf', 'oneOf']) {
		if (
			Array.isArray(schema[key]) &&
			schema[key].some((subSchema: Record<string, any>) => hasOptionalProperties(subSchema))
		) {
			return true
		}
	}

	return false
}

const searchHubScriptsSchema = z.object({
	query: z
		.string()
		.describe('The query to search for, e.g. send email, list stripe invoices, etc..')
})

const searchHubScriptsToolDef = createToolDef(
	searchHubScriptsSchema,
	'search_hub_scripts',
	'Search for scripts in the hub'
)

export const createSearchHubScriptsTool = (withContent: boolean = false) => ({
	def: searchHubScriptsToolDef,
	fn: async ({ args, toolId, toolCallbacks }) => {
		toolCallbacks.setToolStatus(toolId, {
			content: 'Searching for hub scripts related to "' + args.query + '"...'
		})
		const parsedArgs = searchHubScriptsSchema.parse(args)
		const scripts = await ScriptService.queryHubScripts({
			text: parsedArgs.query,
			kind: 'script'
		})
		toolCallbacks.setToolStatus(toolId, {
			content: 'Found ' + scripts.length + ' scripts in the hub related to "' + args.query + '"'
		})
		// if withContent, fetch scripts with their content, limit to 3 results
		const results = await Promise.all(
			scripts.slice(0, withContent ? 3 : undefined).map(async (s) => {
				let content = ''
				if (withContent) {
					content = await ScriptService.getHubScriptContentByPath({
						path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
					})
				}
				return {
					path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: s.summary,
					...(withContent ? { content } : {})
				}
			})
		)
		return JSON.stringify(results)
	}
})

/**
 * Recursively normalizes JSON Schema quirks that specific providers reject.
 */
function normalizeToolParameterSchema(schema: Record<string, any> | undefined): void {
	if (!schema || typeof schema !== 'object') {
		return
	}

	// Remove format if it's null or empty string
	if (schema.format === null || schema.format === '') {
		delete schema.format
	}

	// Recurse into properties
	if (schema.properties && typeof schema.properties === 'object') {
		for (const key of Object.keys(schema.properties)) {
			normalizeToolParameterSchema(schema.properties[key])
		}
	}

	// Recurse into items (for arrays)
	if (schema.items) {
		if (Array.isArray(schema.items)) {
			for (const item of schema.items) {
				normalizeToolParameterSchema(item)
			}
		} else {
			normalizeToolParameterSchema(schema.items)
		}
	}

	// Recurse into additionalProperties if it's an object schema
	if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
		normalizeToolParameterSchema(schema.additionalProperties)
	}

	// Recurse into allOf, anyOf, oneOf
	for (const key of ['allOf', 'anyOf', 'oneOf']) {
		if (Array.isArray(schema[key])) {
			for (const subSchema of schema[key]) {
				normalizeToolParameterSchema(subSchema)
			}
		}
	}
}

export async function buildSchemaForTool(
	toolDef: ChatCompletionFunctionTool,
	schemaBuilder: () => Promise<FunctionParameters>
): Promise<boolean> {
	try {
		const schema = await schemaBuilder()

		// if schema properties contains values different from '^[a-zA-Z0-9_.-]{1,64}$'
		const invalidProperties = Object.keys(schema.properties ?? {}).filter(
			(key) => !/^[a-zA-Z0-9_.-]{1,64}$/.test(key)
		)
		if (invalidProperties.length > 0) {
			console.warn(`Invalid flow inputs schema: ${invalidProperties.join(', ')}`)
			throw new Error(`Invalid flow inputs schema: ${invalidProperties.join(', ')}`)
		}

		// Anthropic requires input_schema.type to be present; flows with no inputs
		// can produce a sparse schema (e.g. { order: [] }) lacking it.
		toolDef.function.parameters = { type: 'object', ...schema, additionalProperties: false }

		// recursively normalize provider-incompatible schema fragments
		normalizeToolParameterSchema(toolDef.function.parameters)

		// OPEN AI models don't support strict mode well with schema with complex properties, so we disable it
		const model = getCurrentModel()
		if (
			model.provider === 'openai' ||
			model.provider === 'azure_openai' ||
			model.provider === 'azure_foundry'
		) {
			toolDef.function.strict = false
		}
		return true
	} catch (error) {
		console.error('Error building schema for tool', error)
		// fallback to schema with args as a JSON string
		toolDef.function.parameters = {
			type: 'object',
			properties: {
				args: { type: 'string', description: 'JSON string containing the arguments for the tool' }
			},
			additionalProperties: false,
			strict: false,
			required: ['args']
		}
		return false
	}
}

// Constants for result formatting
const MAX_RESULT_LENGTH = 12000
const MAX_LOG_LENGTH = 4000
export const MAX_RUNNABLE_CONTENT_LENGTH = 20000

/** How long a test run is awaited inline before it detaches into the background
 * (global/sessions chat only). Quick runs finish well inside this; slow ones are
 * handed to the background poller so the chat loop is freed. */
export const DETACH_AFTER_MS = 15000

export interface TestRunConfig {
	jobStarter: () => Promise<string>
	workspace: string
	toolCallbacks: ToolCallbacks
	toolId: string
	startMessage?: string
	contextName: 'script' | 'flow'
	/** Detach immediately instead of waiting the inline budget (the model's opt-in). */
	background?: boolean
	/** Human label for the jobs tray row (path / step id). Defaults to the job id. */
	label?: string
	/** Overrides the default "…test started, waiting for completion" status while the
	 * job runs inline (e.g. an SQL tool shows "SQL running…"). */
	runningMessage?: string
	/** Custom terminal formatting for callers whose result isn't a plain test-run
	 * summary (e.g. exec_datatable_sql shaping rows). Returns the string handed to
	 * the model plus the tool-card patch. When omitted, the default summary is used. */
	formatCompletion?: (job: CompletedJob) => { llmText: string; card: Partial<ToolDisplayMessage> }
}

// Common job polling function.
//
// Two modes, selected by whether `detachAfterMs` is provided:
//  - Legacy (undefined): poll up to 60×1s, then set a timeout error and throw.
//    Used by in-editor chats, which have no jobs tray to hand off to.
//  - Detach (a number): poll only for that inline budget; if the job is still
//    running when it elapses, resolve `'detached'` instead of throwing so the
//    caller can background the job. `0` detaches without polling at all.
export async function pollJobCompletion(
	jobId: string,
	workspace: string,
	toolId: string,
	toolCallbacks: ToolCallbacks,
	options?: { detachAfterMs?: number }
): Promise<CompletedJob | 'detached'> {
	const detachEnabled = options?.detachAfterMs !== undefined
	const maxAttempts = detachEnabled ? Math.ceil((options?.detachAfterMs ?? 0) / 1000) : 60
	let attempts = 0
	let job: CompletedJob | null = null

	while (attempts < maxAttempts) {
		await new Promise((resolve) => setTimeout(resolve, 1000))
		attempts++

		try {
			const fetchedJob = await JobService.getJob({
				workspace: workspace,
				id: jobId,
				noLogs: false,
				noCode: true
			})

			if (fetchedJob.type === 'CompletedJob') {
				job = fetchedJob
				break
			}
			// Keep the tray's status + Job snapshot fresh during the inline wait.
			toolCallbacks.onJobStatus?.(jobId, {
				status: deriveChatJobStatus(fetchedJob),
				job: trimJob(fetchedJob)
			})
		} catch (error) {
			if (!detachEnabled && attempts >= maxAttempts) {
				throw error
			}
		}
	}

	if (!job) {
		if (detachEnabled) {
			return 'detached'
		}
		toolCallbacks.setToolStatus(toolId, {
			content: 'Test timed out',
			error: 'Execution timed out or failed to complete'
		})
		throw new Error('Test execution timed out after 60 seconds')
	}

	return job
}

// Helper function to extract code blocks from markdown text
export function extractCodeFromMarkdown(markdown: string): string[] {
	const codeBlocks: string[] = []

	// Matches: ```[language]\n[code]\n```
	const codeBlockRegex = /```(?:[a-z]+)?\n([\s\S]*?)```/g

	let match: RegExpExecArray | null = null
	while ((match = codeBlockRegex.exec(markdown)) !== null) {
		const code = match[1].trim()
		if (code) {
			codeBlocks.push(code)
		}
	}

	return codeBlocks
}

// Helper function to get the latest assistant message from display messages
export function getLatestAssistantMessage(displayMessages: DisplayMessage[]): string | undefined {
	// Iterate from the end to find the most recent assistant message
	for (let i = displayMessages.length - 1; i >= 0; i--) {
		const message = displayMessages[i]
		if (message.role === 'assistant' && message.content) {
			return message.content
		}
	}
	return undefined
}

// Helper function to extract error messages from job results
function getErrorMessage(result: unknown): string {
	if (typeof result === 'object' && result !== null && 'error' in result) {
		const error = (result as Record<string, unknown>).error
		if (typeof error === 'object' && error !== null && 'message' in error) {
			const message = (error as Record<string, unknown>).message as string
			if ('stack' in error) {
				return (message + '\n' + (error as Record<string, unknown>).stack) as string
			}
			return message
		}
		if (typeof error === 'string') {
			return error
		}
	}
	if (typeof result === 'string') {
		return result
	}
	return 'Unknown error'
}

// Build test run args based on the tool definition, if it contains a fallback schema
export async function buildTestRunArgs(
	args: any,
	toolDef: ChatCompletionFunctionTool
): Promise<any> {
	let parsedArgs = args
	// if the schema is the fallback schema, parse the args as a JSON string
	if (
		(toolDef.function.parameters as any).properties?.args?.description ===
		'JSON string containing the arguments for the tool'
	) {
		try {
			parsedArgs = JSON.parse(args.args)
		} catch (error) {
			console.error('Error parsing arguments for tool', error)
		}
	}
	return parsedArgs
}

// The string handed back to the model when a job is backgrounded. It carries the
// job id so the model can pull status/logs on demand (get_job_logs / list_runs),
// and tells it the completion will be reported later (notify-only wake).
function backgroundedSummary(jobId: string, label: string): string {
	return (
		`Job ${jobId} for "${label}" is taking a while and is now running in the background — ` +
		`the chat is free to continue and you'll be told when it finishes. ` +
		`To inspect it now, call get_job_logs with id="${jobId}" (or list_runs); ` +
		`to stop it, call cancel_job with id="${jobId}".`
	)
}

// Tool-card status patch for a completed background job. Mirrors the inline
// terminal branch of executeTestRun so a job that finished in the background
// fills its card the same way one that finished inline does.
export function completedJobToolStatus(job: CompletedJob): Partial<ToolDisplayMessage> {
	return {
		content: `Background job ${job.success ? 'completed successfully' : 'failed'}`,
		result: formatResult(job.result),
		logs: formatLogs(job.logs),
		...(job.success ? {} : { error: getErrorMessage(job.result) })
	}
}

// Short completion note handed to the model on its next turn (notify-only wake).
// Carries the id so the model can pull full logs via get_job_logs on demand.
export function backgroundJobCompletionNote(
	jobId: string,
	label: string,
	job: CompletedJob
): string {
	const status = job.success ? 'succeeded' : 'FAILED'
	const resultHead = formatResult(job.result).slice(0, 2000)
	return (
		`Background job ${jobId} for "${label}" ${status}.\n` +
		`Result: ${resultHead}\n` +
		`(For full logs call get_job_logs with id="${jobId}".)`
	)
}

// Main execution function for test runs
export async function executeTestRun(config: TestRunConfig): Promise<string> {
	// Detach-into-background is enabled only when the host wired the job hooks
	// (global/sessions chat). Otherwise this stays a blocking call.
	const detachEnabled = !!config.toolCallbacks.onJobStarted
	const label = config.label ?? config.contextName
	try {
		config.toolCallbacks.setToolStatus(config.toolId, {
			content: config.startMessage || `Starting ${config.contextName} test...`
		})

		const jobId = await config.jobStarter()

		const contextName = config.contextName.charAt(0).toUpperCase() + config.contextName.slice(1)

		// Register the job so the tray shows it from the moment it is queued.
		config.toolCallbacks.onJobStarted?.({
			jobId,
			toolCallId: config.toolId,
			kind: config.contextName,
			label,
			workspace: config.workspace
		})

		config.toolCallbacks.setToolStatus(config.toolId, {
			content: config.runningMessage ?? `${contextName} test started, waiting for completion...`
		})

		const outcome = await pollJobCompletion(
			jobId,
			config.workspace,
			config.toolId,
			config.toolCallbacks,
			detachEnabled ? { detachAfterMs: config.background ? 0 : DETACH_AFTER_MS } : undefined
		)

		if (outcome === 'detached') {
			config.toolCallbacks.onJobDetached?.(jobId)
			config.toolCallbacks.setToolStatus(config.toolId, {
				content: `${contextName} test running in background (job ${jobId})`
			})
			return backgroundedSummary(jobId, label)
		}

		const job = outcome
		config.toolCallbacks.onJobStatus?.(jobId, {
			status: deriveChatJobStatus(job),
			durationMs: job.duration_ms,
			job: trimJob(job)
		})

		if (config.formatCompletion) {
			const { llmText, card } = config.formatCompletion(job)
			config.toolCallbacks.setToolStatus(config.toolId, card)
			return llmText
		}

		config.toolCallbacks.setToolStatus(config.toolId, {
			content: `${contextName} test ${job.success ? 'completed successfully' : 'failed'}`,
			result: formatResult(job.result),
			logs: formatLogs(job.logs),
			...(job.success ? {} : { error: getErrorMessage(job.result) })
		})

		return formatResultSummary(job.result, job.logs, job.success)
	} catch (error) {
		const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
		config.toolCallbacks.setToolStatus(config.toolId, {
			content: `Test execution failed`,
			error: errorMessage
		})
		throw new Error(`Failed to execute test run: ${errorMessage}`)
	}
}

type FlowStepScriptLoader = (
	moduleValue: { path: string; hash?: string },
	workspace: string
) => Promise<{ content: string; language: ScriptLang }>

type FlowStepPreviewLoader = (path: string, workspace: string) => Promise<FlowValue | undefined>

export type FlowStepTestRunConfig = {
	flowValue: FlowValue
	stepId: string
	args?: Record<string, any> | null
	workspace: string
	toolCallbacks: ToolCallbacks
	toolId: string
	background?: boolean
	loadScript?: FlowStepScriptLoader
	loadFlowPreviewValue?: FlowStepPreviewLoader
}

function normalizeFlowStepArgs(args: Record<string, any> | null | undefined): Record<string, any> {
	return args ?? {}
}

function flowStepArgsForModule(moduleId: string, args: Record<string, any>): Record<string, any> {
	return moduleId === SPECIAL_MODULE_IDS.PREPROCESSOR
		? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args }
		: args
}

function getAvailableFlowStepIds(flowValue: FlowValue): string {
	return Array.from(
		new Set([
			...extractAllModules(flowValue.modules ?? []).map((module: FlowModule) => module.id),
			...(flowValue.preprocessor_module ? [flowValue.preprocessor_module.id] : []),
			...(flowValue.failure_module ? [flowValue.failure_module.id] : [])
		])
	).join(', ')
}

async function loadDeployedScriptForFlowStep(
	moduleValue: { path: string; hash?: string },
	workspace: string
): Promise<{ content: string; language: ScriptLang }> {
	const script = moduleValue.hash
		? await ScriptService.getScriptByHash({ workspace, hash: moduleValue.hash })
		: await ScriptService.getScriptByPath({ workspace, path: moduleValue.path })
	return { content: script.content, language: script.language }
}

export async function executeFlowStepTestRun({
	flowValue,
	stepId,
	args,
	workspace,
	toolCallbacks,
	toolId,
	background,
	loadScript = loadDeployedScriptForFlowStep,
	loadFlowPreviewValue
}: FlowStepTestRunConfig): Promise<string> {
	const targetModule = findModuleInFlow(flowValue, stepId) ?? undefined

	if (!targetModule) {
		toolCallbacks.setToolStatus(toolId, {
			content: `Step "${stepId}" not found in flow`,
			error: `Step with id "${stepId}" does not exist in the current flow`
		})
		throw new Error(
			`Step with id "${stepId}" not found in flow. Available steps: ${getAvailableFlowStepIds(flowValue)}`
		)
	}

	const moduleValue = targetModule.value
	const stepArgs = normalizeFlowStepArgs(args)

	if (moduleValue.type === 'rawscript') {
		return executeTestRun({
			jobStarter: () =>
				JobService.runScriptPreview({
					workspace,
					requestBody: {
						content: moduleValue.content ?? '',
						language: moduleValue.language,
						args: flowStepArgsForModule(targetModule.id, stepArgs)
					}
				}),
			workspace,
			toolCallbacks,
			toolId,
			startMessage: `Starting test run of step "${stepId}"...`,
			contextName: 'script',
			label: `step ${stepId}`,
			background
		})
	}

	if (moduleValue.type === 'script') {
		const script = await loadScript(moduleValue, workspace)
		return executeTestRun({
			jobStarter: () =>
				JobService.runScriptPreview({
					workspace,
					requestBody: {
						path: moduleValue.path,
						content: script.content,
						language: script.language,
						args: flowStepArgsForModule(targetModule.id, stepArgs)
					}
				}),
			workspace,
			toolCallbacks,
			toolId,
			startMessage: `Starting test run of script step "${stepId}"...`,
			contextName: 'script',
			label: `step ${stepId}`,
			background
		})
	}

	if (moduleValue.type === 'flow') {
		const previewValue = await loadFlowPreviewValue?.(moduleValue.path, workspace)
		if (previewValue) {
			return executeTestRun({
				jobStarter: () =>
					JobService.runFlowPreview({
						workspace,
						requestBody: {
							path: moduleValue.path,
							value: previewValue,
							args: stepArgs
						}
					}),
				workspace,
				toolCallbacks,
				toolId,
				startMessage: `Starting test run of draft flow step "${stepId}"...`,
				contextName: 'flow',
				label: `step ${stepId}`,
				background
			})
		}

		return executeTestRun({
			jobStarter: () =>
				JobService.runFlowByPath({
					workspace,
					path: moduleValue.path,
					requestBody: stepArgs
				}),
			workspace,
			toolCallbacks,
			toolId,
			startMessage: `Starting test run of flow step "${stepId}"...`,
			contextName: 'flow',
			label: `step ${stepId}`,
			background
		})
	}

	toolCallbacks.setToolStatus(toolId, {
		content: `Step type "${moduleValue.type}" not supported for testing`,
		error: `Cannot test step of type "${moduleValue.type}"`
	})
	throw new Error(
		`Cannot test step of type "${moduleValue.type}". Supported types: rawscript, script, flow`
	)
}

function formatLogs(logs: string | undefined): undefined | string {
	if (logs && logs.trim()) {
		if (logs.length <= MAX_LOG_LENGTH) {
			return logs
		} else {
			return logs.slice(-MAX_LOG_LENGTH)
		}
	}
	return undefined
}

function formatResult(result: unknown): string {
	if (typeof result === 'string') {
		return result
	}
	return JSON.stringify(result, null, 2)
}

function formatResultSummary(result: unknown, logs: string | undefined, success: boolean): string {
	let resultSummary = ''
	resultSummary += `Result (${success ? 'SUCCESS' : 'FAILED'})\n\n`
	resultSummary += formatResult(result).slice(0, MAX_RESULT_LENGTH)
	resultSummary += '\n\nLogs:\n\n'
	resultSummary += formatLogs(logs) ?? 'No logs available'
	return resultSummary
}

// ============= Script/Flow Lint Types =============

/** Result of linting a script */
export interface ScriptLintResult {
	errorCount: number
	warningCount: number
	errors: meditor.IMarker[]
	warnings: meditor.IMarker[]
}

/** Format script lint result for display */
export function formatScriptLintResult(lintResult: ScriptLintResult): string {
	let response = ''
	const hasIssues = lintResult.errorCount > 0 || lintResult.warningCount > 0

	if (hasIssues) {
		if (lintResult.errorCount > 0) {
			response += `❌ **${lintResult.errorCount} error(s)** found that must be fixed:\n`
			for (const error of lintResult.errors) {
				response += `- Line ${error.startLineNumber}: ${error.message}\n`
			}
		}

		if (lintResult.warningCount > 0) {
			response += `\n⚠️ **${lintResult.warningCount} warning(s)** found:\n`
			for (const warning of lintResult.warnings) {
				response += `- Line ${warning.startLineNumber}: ${warning.message}\n`
			}
		}
	} else {
		response = '✅ No lint issues found.'
	}

	return response
}

export class WorkspaceRunnablesSearch {
	private uf: uFuzzy
	private scriptsWorkspace: string | undefined = undefined
	private flowsWorkspace: string | undefined = undefined
	private scripts: Script[] | undefined = undefined
	private flows: Flow[] | undefined = undefined
	private scriptCache: Map<string, Awaited<ReturnType<typeof ScriptService.getScriptByPath>>> =
		new Map()
	private flowCache: Map<string, Awaited<ReturnType<typeof FlowService.getFlowByPath>>> = new Map()

	constructor() {
		this.uf = new uFuzzy()
	}

	private async initScripts(workspace: string) {
		if (this.scripts === undefined || this.scriptsWorkspace !== workspace) {
			this.scripts = await ScriptService.listScripts({ workspace })
			this.scriptsWorkspace = workspace
		}
	}

	private async initFlows(workspace: string) {
		if (this.flows === undefined || this.flowsWorkspace !== workspace) {
			this.flows = await FlowService.listFlows({ workspace })
			this.flowsWorkspace = workspace
		}
	}

	async searchScripts(query: string, workspace: string) {
		await this.initScripts(workspace)
		const scripts = this.scripts
		if (!scripts) return []

		const trimmed = query.trim()
		if (!trimmed) {
			return scripts.map((s) => ({
				type: 'script' as const,
				path: s.path,
				summary: s.summary
			}))
		}

		const haystack = scripts.map((s) =>
			emptyString(s.summary) ? s.path : s.summary + ' (' + s.path + ')'
		)
		const [idxs, , order] = this.uf.search(haystack, trimmed)
		if (!idxs || !order) return []
		return order.map((orderIdx) => {
			const haystackIdx = idxs[orderIdx]
			return {
				type: 'script' as const,
				path: scripts[haystackIdx].path,
				summary: scripts[haystackIdx].summary
			}
		})
	}

	async searchFlows(query: string, workspace: string) {
		await this.initFlows(workspace)
		const flows = this.flows
		if (!flows) return []

		const trimmed = query.trim()
		if (!trimmed) {
			return flows.map((f) => ({
				type: 'flow' as const,
				path: f.path,
				summary: f.summary
			}))
		}

		const haystack = flows.map((f) =>
			emptyString(f.summary) ? f.path : f.summary + ' (' + f.path + ')'
		)
		const [idxs, , order] = this.uf.search(haystack, trimmed)
		if (!idxs || !order) return []
		return order.map((orderIdx) => {
			const haystackIdx = idxs[orderIdx]
			return {
				type: 'flow' as const,
				path: flows[haystackIdx].path,
				summary: flows[haystackIdx].summary
			}
		})
	}

	async search(query: string, workspace: string, type: 'all' | 'scripts' | 'flows' = 'all') {
		const results: { type: 'script' | 'flow'; path: string; summary: string }[] = []

		if (type === 'all' || type === 'scripts') {
			results.push(...(await this.searchScripts(query, workspace)))
		}
		if (type === 'all' || type === 'flows') {
			results.push(...(await this.searchFlows(query, workspace)))
		}

		return results
	}

	async getScript(path: string, workspace: string) {
		const key = `${workspace}:${path}`
		let cached = this.scriptCache.get(key)
		if (!cached) {
			cached = await ScriptService.getScriptByPath({ workspace, path })
			this.scriptCache.set(key, cached)
		}
		return cached
	}

	async getFlow(path: string, workspace: string) {
		const key = `${workspace}:${path}`
		let cached = this.flowCache.get(key)
		if (!cached) {
			cached = await FlowService.getFlowByPath({ workspace, path })
			this.flowCache.set(key, cached)
		}
		return cached
	}
}

const searchWorkspaceSchema = z.object({
	query: z
		.string()
		.describe('Comma separated list of keywords to search for (e.g. "stripe, send email, ETL")'),
	type: z
		.enum(['all', 'scripts', 'flows'])
		.describe(
			'Filter by type: "all" for both scripts and flows, "scripts" for scripts only, "flows" for flows only.'
		)
})

const searchWorkspaceToolDef = createToolDef(
	searchWorkspaceSchema,
	'search_workspace',
	'Search for scripts and flows in the workspace. Use this when a user asks about existing building blocks, wants to find a script/flow, or asks "what do I have for X". ALWAYS search really broadly.'
)

export const workspaceRunnablesSearch = new WorkspaceRunnablesSearch()

export const createSearchWorkspaceTool = () => ({
	def: searchWorkspaceToolDef,
	fn: async ({
		args,
		workspace,
		toolId,
		toolCallbacks
	}: {
		args: any
		workspace: string
		toolId: string
		toolCallbacks: ToolCallbacks
	}) => {
		const parsedArgs = searchWorkspaceSchema.parse(args)
		const type = parsedArgs.type
		toolCallbacks.setToolStatus(toolId, {
			content: `Searching workspace...`
		})

		const results: { type: 'script' | 'flow'; path: string; summary: string }[] = []
		const keywords = parsedArgs.query.split(',').map((keyword) => keyword.trim())
		const seenPaths = new Set<string>()
		for (const keyword of keywords) {
			const keywordResults = await workspaceRunnablesSearch.search(keyword, workspace, type)
			for (const result of keywordResults) {
				if (!seenPaths.has(result.path)) {
					results.push(result)
					seenPaths.add(result.path)
				}
			}
		}

		toolCallbacks.setToolStatus(toolId, {
			content: `Found ${results.length} result(s)`
		})
		return JSON.stringify(results, null, 2)
	}
})

const getRunnableDetailsSchema = z.object({
	path: z.string().describe('The path of the script or flow (e.g. "f/marketing/send_email")'),
	type: z.enum(['script', 'flow']).describe('Whether this is a script or a flow')
})

const getRunnableDetailsToolDef = createToolDef(
	getRunnableDetailsSchema,
	'get_runnable_details',
	'Get details (summary, description, inputs schema, content) of a specific script or flow by path'
)

export const createGetRunnableDetailsTool = () => ({
	def: getRunnableDetailsToolDef,
	fn: async ({
		args,
		workspace,
		toolId,
		toolCallbacks
	}: {
		args: any
		workspace: string
		toolId: string
		toolCallbacks: ToolCallbacks
	}) => {
		const parsedArgs = getRunnableDetailsSchema.parse(args)
		const { path, type } = parsedArgs
		toolCallbacks.setToolStatus(toolId, {
			content: `Getting ${type} details for "${path}"...`
		})

		try {
			if (type === 'script') {
				const script = await workspaceRunnablesSearch.getScript(path, workspace)
				toolCallbacks.setToolStatus(toolId, {
					content: `Retrieved script details for "${path}"`
				})
				const content = script.content ?? ''
				const truncatedContent =
					content.length > MAX_RUNNABLE_CONTENT_LENGTH
						? content.slice(0, MAX_RUNNABLE_CONTENT_LENGTH) + '\n... (truncated)'
						: content
				return JSON.stringify(
					{
						path: script.path,
						summary: script.summary,
						description: script.description,
						language: script.language,
						schema: script.schema,
						content: truncatedContent
					},
					null,
					2
				)
			} else {
				const flow = await workspaceRunnablesSearch.getFlow(path, workspace)
				toolCallbacks.setToolStatus(toolId, {
					content: `Retrieved flow details for "${path}"`
				})
				const flowValue = JSON.stringify(flow.value, null, 2)
				const truncatedValue =
					flowValue.length > MAX_RUNNABLE_CONTENT_LENGTH
						? flowValue.slice(0, MAX_RUNNABLE_CONTENT_LENGTH) + '\n... (truncated)'
						: flowValue
				return JSON.stringify(
					{
						path: flow.path,
						summary: flow.summary,
						description: flow.description,
						schema: flow.schema,
						value: truncatedValue
					},
					null,
					2
				)
			}
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error)
			toolCallbacks.setToolStatus(toolId, {
				content: `Error getting ${type} details`,
				error: errorMessage
			})
			return `Error getting ${type} details for "${path}": ${errorMessage}`
		}
	}
})
