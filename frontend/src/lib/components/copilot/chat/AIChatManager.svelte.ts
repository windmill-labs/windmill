import type { ScriptLang } from '$lib/gen/types.gen'
import { WorkspaceService, JobService, type CompletedJob } from '$lib/gen'
import type { FlowOptions, ScriptOptions } from './ContextManager.svelte'
import {
	flowTools,
	prepareFlowSystemMessage,
	prepareFlowUserMessage,
	type FlowAIChatHelpers
} from './flow/core'
import {
	getAppTools,
	prepareAppSystemMessage,
	prepareAppUserMessage,
	type AppAIChatHelpers
} from './app/core'
import ContextManager from './ContextManager.svelte'
import HistoryManager from './HistoryManager.svelte'
import {
	type DisplayMessage,
	type Tool,
	type ToolCallbacks,
	type ToolDisplayMessage,
	type UserQuestionDisplay,
	type ChatJob,
	type ChatJobInit,
	type ChatJobStatus,
	completedJobToolStatus,
	backgroundJobCompletionNote,
	deriveChatJobStatus,
	trimJob
} from './shared'
import type {
	ChatCompletionMessageParam,
	ChatCompletionSystemMessageParam
} from 'openai/resources/chat/completions.mjs'
import {
	prepareInlineChatSystemPrompt,
	prepareScriptSystemMessage,
	prepareScriptTools
} from './script/core'
import type { ScriptLintResult } from './shared'
import { navigatorTools, prepareNavigatorSystemMessage } from './navigator/core'
import { loadApiTools } from './api/apiTools'
import { prepareScriptUserMessage } from './script/core'
import { prepareNavigatorUserMessage } from './navigator/core'
import { sendUserToast } from '$lib/toast'
import { workspaceAIClients, getNonStreamingCompletion } from '../lib'
import { getKnownModelContextWindow } from '../modelConfig'
import {
	getCompactionSummaryPrompt,
	formatCompactSummary,
	buildSummaryMessageContent
} from './compactionPrompt'
import { dfs } from '$lib/components/flows/previousResults'
import { SvelteSet } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'
import { maskKey } from '$lib/components/sessions/modifiedItemsMask'
import { getStringError } from './utils'
import { type PasteAttachment } from './pasteTokens'
import { chatDraft, expanded } from './chatDraft'
import type { FlowModuleState, FlowState } from '$lib/components/flows/flowState'
import type { CurrentEditor, ExtendedOpenFlow } from '$lib/components/flows/types'
import { untrack } from 'svelte'
import { get } from 'svelte/store'
import { BROWSER } from 'esm-env'
import { workspaceStore, type DBSchemas } from '$lib/stores'
import { copilotInfo, copilotWorkspaceRequested, loadCopilot } from '$lib/aiStore'
import { askTools, prepareAskSystemMessage, prepareAskUserMessage } from './ask/core'
import { readDocsPageTool, searchDocsTool } from './docs/core'
import { TypewriterReveal } from './typewriterReveal'
import { chatState, DEFAULT_SIZE, triggerablesByAi } from './sharedChatState.svelte'
import {
	createAppBackendRunnableContextElement,
	createAppFrontendFileContextElement,
	flattenDatatablesToAppContextElements,
	type ContextElement,
	type AppDatatableElement
} from './context'
import type { Selection } from 'monaco-editor'
import type AIChatInput from './AIChatInput.svelte'
import { prepareApiSystemMessage, prepareApiUserMessage } from './api/core'
import { runChatLoop, truncateToToolPairedPrefix } from './chatLoop'
import { sanitizeToolCallArguments } from './toolCallArguments'
import { normalizeContextUsage } from './tokenUsage'
import type { ReviewChangesOpts } from './monaco-adapter'
import {
	getCurrentModel,
	tryGetCurrentModel,
	getCombinedCustomPrompt,
	getCustomPromptParts,
	getUserCustomPrompts,
	setUserCustomPrompts,
	isWebSearchEnabledForProvider
} from '$lib/aiStore'
import type { WorkspaceMutationTarget } from './workspaceTools'
import {
	globalToolsFor,
	loadWorkspaceSkills,
	prepareGlobalSystemMessage,
	prepareGlobalUserMessage,
	type AiSkillListItem,
	type GlobalToolHelpers
} from './global/core'
import { formatChatJobCompletion } from './datatableTools'
import { isGlobalAiEnabled } from './global/gate'
import {
	pipelineTools,
	getPipelinePromptSection,
	type PipelineAIChatHelpers
} from './pipeline/core'
import { scopedKey, onUserChange, migrateLegacyLocalStorage } from '$lib/userScopedStorage'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { AttachedFilesStore } from './files/attachedFiles.svelte'
import { appendAttachedFilesRoster } from './files/fileTools'

// SSR and users who prefer reduced motion get no typewriter pacing.
function prefersInstantReveal(): boolean {
	return !BROWSER || (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false)
}

// Compaction of the stored history: once the projected request size
// (contextTokens — the provider's report when current, a fresh chars/4
// estimate otherwise — plus the new user message) reaches the trigger ratio of
// the model's context window, the older prefix is summarized into a single
// message while the recent tail is kept verbatim, bringing the history down to
// roughly the target ratio. The trigger headroom absorbs what the projection
// cannot see — the upcoming completion and tool results, system-prompt/tool-
// schema changes from mode switches, and the estimate's chars/4 error.
const COMPACTION_TRIGGER_RATIO = 0.8
const COMPACTION_TARGET_RATIO = 0.7
// Headroom reserved within the target budget for the summary message itself, so
// the summary + kept tail + overhead land under the target ratio.
const SUMMARY_OUTPUT_RESERVE_TOKENS = 8000
// Below this many messages in the prefix there's little to gain from a summary
// round-trip; skip straight to drop-oldest.
const MIN_PREFIX_MESSAGES_TO_SUMMARIZE = 4
// Stop attempting summarization after this many consecutive failures and use
// drop-oldest directly; a successful summarization resets the counter.
const MAX_CONSECUTIVE_COMPACTION_FAILURES = 3
// Abort reason for a deliberate user cancel (Esc / Stop). Programmatic cancels
// (panel teardown, save-and-clear) pass their own reason, so the queued-message
// flush can tell "the user wants to move on" from "the turn was torn down".
const USER_CANCEL_REASON = 'user_cancelled'
// Built-in `/compact` session command — summarizes the conversation locally
// instead of sending a turn to the model. Matched on the whole input so a
// regular message that merely mentions "/compact" mid-sentence is unaffected.
const COMPACT_COMMAND_NAME = 'compact'
const COMPACT_COMMAND_RE = /^\/compact\s*$/
// Built-in `/clear` session command — saves the conversation to history and
// resets to a fresh chat (the "New chat" action), instead of sending a turn.
const CLEAR_COMMAND_NAME = 'clear'
const CLEAR_COMMAND_RE = /^\/clear\s*$/
const AI_AUTONOMY_MODE_STORAGE_KEY = 'ai-chat-autonomy-mode'
const LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY = 'ai-chat-yolo-mode'
const WEB_SEARCH_ERROR_HINT =
	'Web search is unavailable for this provider/model/key. Disable web search in workspace settings and try again.'

export enum AIMode {
	SCRIPT = 'script',
	FLOW = 'flow',
	APP = 'app',
	NAVIGATOR = 'navigator',
	API = 'API',
	GLOBAL = 'global',
	ASK = 'ask'
}

export enum AIAutonomyMode {
	DEFAULT = 'default',
	ACCEPT_EDIT = 'acceptedit',
	YOLO = 'yolo'
}

const ALL_AI_MODES = Object.values(AIMode)
const ALL_AI_AUTONOMY_MODES = Object.values(AIAutonomyMode)
const AUTO_ACCEPT_EDIT_MODES = new Set<AIMode>([AIMode.SCRIPT, AIMode.FLOW])
const AUTO_ACCEPT_TOOL_CONFIRMATION_MODES = new Set<AIMode>([
	AIMode.SCRIPT,
	AIMode.FLOW,
	AIMode.APP,
	AIMode.GLOBAL
])

export function isAIMode(mode: unknown): mode is AIMode {
	return ALL_AI_MODES.includes(mode as AIMode)
}

export function isAIAutonomyMode(mode: unknown): mode is AIAutonomyMode {
	return ALL_AI_AUTONOMY_MODES.includes(mode as AIAutonomyMode)
}

export function supportsAutoAcceptEdits(mode: AIMode): boolean {
	return AUTO_ACCEPT_EDIT_MODES.has(mode)
}

export function supportsAutoAcceptToolConfirmations(mode: AIMode): boolean {
	return AUTO_ACCEPT_TOOL_CONFIRMATION_MODES.has(mode)
}

export function isAIModeVisible(mode: AIMode): boolean {
	return mode !== AIMode.GLOBAL || isGlobalAiEnabled()
}

export function getVisibleAIModes(): AIMode[] {
	return ALL_AI_MODES.filter(isAIModeVisible)
}

function isWorkspacePath(path: string | undefined): path is string {
	return path?.startsWith('f/') === true || path?.startsWith('u/') === true
}

// The autonomy mode is namespaced by the logged-in user's email (scopedKey).
// It controls whether tool calls auto-execute, so leaking it across users on a
// shared browser is a safety concern (user B inheriting user A's YOLO mode).
// Returns the safe ACCEPT_EDIT default when no user is known yet — the
// module-level singleton (constructed at import, before the email resolves)
// re-reads via onUserChange once it does.
function getPersistedAutonomyMode(): AIAutonomyMode {
	const key = scopedKey(AI_AUTONOMY_MODE_STORAGE_KEY)
	if (!BROWSER || !key) {
		return AIAutonomyMode.ACCEPT_EDIT
	}
	const persistedMode = getLocalSetting(key)
	if (isAIAutonomyMode(persistedMode)) {
		return persistedMode
	}
	// No stored preference: default to auto-accepting edits (tool calls still
	// require confirmation; only YOLO bypasses those). Note this means users who
	// never opened the autonomy picker now start with edit auto-accept on.
	const legacyKey = scopedKey(LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY)
	return legacyKey && getLocalSetting(legacyKey) === 'true'
		? AIAutonomyMode.YOLO
		: AIAutonomyMode.ACCEPT_EDIT
}

function persistAutonomyMode(mode: AIAutonomyMode) {
	const key = scopedKey(AI_AUTONOMY_MODE_STORAGE_KEY)
	if (!BROWSER || !key) {
		return
	}
	storeLocalSetting(key, mode)
}

// Claim the pre-namespacing autonomy keys for the first user to log in on a
// previously single-user browser.
function migrateLegacyAutonomyKeys() {
	migrateLegacyLocalStorage(AI_AUTONOMY_MODE_STORAGE_KEY, scopedKey(AI_AUTONOMY_MODE_STORAGE_KEY))
	migrateLegacyLocalStorage(
		LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY,
		scopedKey(LEGACY_AUTO_ACCEPT_TOOL_CONFIRMATIONS_STORAGE_KEY)
	)
}

function appendWebSearchErrorHint(message: string, shouldAppend: boolean): string {
	if (!shouldAppend) {
		return message
	}
	const separator = /[.!?]$/.test(message.trim()) ? ' ' : '. '
	return `${message}${separator}${WEB_SEARCH_ERROR_HINT}`
}

function getSendRequestErrorMessage(err: unknown, webSearchUnavailable: boolean): string {
	const errorMessage =
		err instanceof Error ? err.message : typeof err === 'string' ? err : undefined
	const message = errorMessage
		? `Failed to send request: ${errorMessage}`
		: 'Failed to send request'
	return appendWebSearchErrorHint(message, webSearchUnavailable)
}

/**
 * Re-fetch copilotInfo after a free-tier turn so its `freeTier.used_ratio` (and the usage
 * banner above the composer that reads it) tracks spend live, and so the turn that finally
 * exhausts the grant flips `freeTier.exhausted` — revealing the banner — instead of the
 * user seeing only a per-request error toast until a page reload. copilotInfo is otherwise
 * fetched only on workspace load.
 *
 * Scoped to users actually on the free tier and not already exhausted, so it costs one
 * extra request only while the grant is live, and never for configured-key users.
 * Keyed on that state rather than on matching an error message, which would break the
 * moment the copy changes.
 */
async function refreshFreeTierUsage(workspace: string | undefined) {
	if (!workspace) return
	// The global copilotInfo/client are a singleton shared by every mounted session. A warm
	// session can finish a turn after the user has switched to another workspace; refreshing
	// then would call loadCopilot for the completing manager's (now background) workspace and
	// clobber the active one's models/client. Compare against the most-recently-*requested*
	// workspace (set synchronously), not the last-*resolved* one — otherwise a refresh could
	// fire while a newer workspace's load is still in flight, win the monotonic token, and
	// restore this (stale) workspace over it.
	if (get(copilotWorkspaceRequested) !== workspace) return
	const info = get(copilotInfo)
	if (!info.freeTier || info.freeTier.exhausted) return
	try {
		await loadCopilot(workspace)
	} catch (err) {
		console.error('Failed to refresh free-tier usage', err)
	}
}

export class AIChatManager {
	contextManager = new ContextManager()
	historyManager = new HistoryManager()
	/** Files the user attached to the current GLOBAL-mode conversation. */
	attachedFiles = new AttachedFilesStore()
	abortController: AbortController | undefined = undefined
	inlineAbortController: AbortController | undefined = undefined
	// Flag to skip Responses API if it's not available (e.g., Azure region doesn't support it)
	skipResponsesApi = false

	mode = $state<AIMode>(AIMode.NAVIGATOR)
	pipelineAiChatHelpers = $state<PipelineAIChatHelpers | undefined>(undefined)
	readonly isOpen = $derived(chatState.size > 0)
	savedSize = $state<number>(0)
	instructions = $state<string>('')
	pendingPrompt = $state<string>('')
	// Message typed while a turn is streaming. There is only ever one queued
	// message; pressing Enter again appends another line to it. Auto-sent when
	// the turn finishes (clean completion or user cancel). Ephemeral — never
	// saved to displayMessages or history.
	queuedMessage = $state<string>('')
	// Jobs the chat started that detached into the background (global/sessions
	// chat only). Rendered in the jobs tray, persisted with the chat, and advanced
	// by a single background poller. See registerJob / #pollBackgroundJobs.
	backgroundJobs = $state<ChatJob[]>([])
	// Completion notes for finished background jobs awaiting delivery to the model.
	// Drained as a preamble into the next turn — either the user's next message, or,
	// when the chat is idle, an auto-resume turn started for them (see
	// #maybeAutoResumeFromJobs). Ephemeral like queuedMessage — not persisted.
	pendingJobNotes = $state<string[]>([])
	// Guards #maybeAutoResumeFromJobs against re-entering while its own turn spins up.
	#autoResuming = false
	#jobPollTimer: ReturnType<typeof setTimeout> | undefined = undefined
	#jobPollDelay = 2000
	// True while a #pollBackgroundJobs pass is executing. #stopJobPoller only clears
	// the scheduled timer, not an in-flight poll, so without this a refreshBackgroundJobs
	// mid-poll (cancel / approval close) would start a second concurrent poll chain and
	// double the poll rate. The guard makes such a refresh coalesce into the running pass.
	#isPolling = false
	// Bumped on every conversation switch (clearBackgroundJobs). An in-flight poll
	// captures it before its awaits and bails if it changed, so a getJob that
	// resolves after the user switched chats can't mutate the newly-loaded one.
	#jobPollGeneration = 0
	// Consecutive getJob failures per background job, so a vanished/404 job can be
	// drained instead of polled forever. Ephemeral, keyed by jobId.
	#jobPollFailures = new Map<string, number>()
	/** Opens a run in the sessions preview pane. Set by the session runtime;
	 * undefined in the global side-panel chat, where the tray falls back to opening
	 * the run in a new browser tab. */
	openRunInPreview?: (a: { jobId: string; workspace: string; label: string }) => void
	loading = $state<boolean>(false)
	currentReply = $state<string>('')
	currentReasoning = $state<string>('')
	currentReasoningActive = $state<boolean>(false)
	// Smooths the provider's bursty delivery into continuous typing by revealing
	// buffered text a slice per frame. The reply and the reasoning/thinking stream
	// each get their own reveal (independent buffers, both append to their own
	// $state). Reduced-motion (sampled once — the pref never changes mid-session)
	// and SSR fall back to instant.
	private replyReveal = new TypewriterReveal({
		onReveal: (chunk) => (this.currentReply += chunk),
		instant: prefersInstantReveal()
	})
	private reasoningReveal = new TypewriterReveal({
		onReveal: (chunk) => (this.currentReasoning += chunk),
		instant: prefersInstantReveal()
	})
	displayMessages = $state<DisplayMessage[]>([])
	messages = $state<ChatCompletionMessageParam[]>([])
	/** Provider-reported context size of the last committed turn (prompt +
	 * completion of its latest completion — exact, includes system prompt and
	 * tools), or undefined whenever no report describes the current history
	 * (provider never reported, turn failed, history rewound). Never holds a
	 * guess: readers go through `contextTokens`, which estimates lazily. */
	contextUsage = $state<number | undefined>(undefined)
	// Circuit breaker for summary-based compaction: after repeated failures the
	// summary round-trip is skipped in favor of drop-oldest. Reset on any
	// successful summarization. Not persisted — a fresh load gets a fresh chance.
	private consecutiveCompactionFailures = 0
	// True while the summarization round-trip is in flight, so the UI can show a
	// "Compacting conversation" label on the processing indicator.
	compacting = $state(false)
	// General-purpose label for the processing indicator, set by a beforeSend hook
	// to describe pre-flight work (e.g. "Creating workspace fork...") that runs
	// before the request goes out. Takes precedence over the compacting/thinking
	// labels while set; the hook clears it back to undefined when done.
	loadingLabel = $state<string | undefined>(undefined)
	autonomyMode = $state<AIAutonomyMode>(getPersistedAutonomyMode())
	autoAcceptEditsAvailable = $derived(supportsAutoAcceptEdits(this.mode))
	autoAcceptEditsActive = $derived(
		this.autoAcceptEditsAvailable &&
			(this.autonomyMode === AIAutonomyMode.ACCEPT_EDIT ||
				this.autonomyMode === AIAutonomyMode.YOLO)
	)
	autoAcceptToolConfirmationsAvailable = $derived(supportsAutoAcceptToolConfirmations(this.mode))
	autoAcceptToolConfirmationsActive = $derived(
		this.autonomyMode === AIAutonomyMode.YOLO && this.autoAcceptToolConfirmationsAvailable
	)
	#automaticScroll = $state<boolean>(true)
	systemMessage = $state<ChatCompletionSystemMessageParam>({
		role: 'system',
		content: ''
	})
	tools = $state<Tool<any>[]>([])
	helpers = $state<any | undefined>(undefined)

	scriptEditorOptions = $state<ScriptOptions | undefined>(undefined)
	flowOptions = $state<FlowOptions | undefined>(undefined)
	scriptEditorApplyCode = $state<
		((code: string, opts?: ReviewChangesOpts) => void | Promise<void>) | undefined
	>(undefined)
	scriptEditorShowDiffMode = $state<(() => void) | undefined>(undefined)
	scriptEditorGetLintErrors = $state<(() => ScriptLintResult) | undefined>(undefined)
	flowAiChatHelpers = $state<FlowAIChatHelpers | undefined>(undefined)
	appAiChatHelpers = $state<AppAIChatHelpers | undefined>(undefined)
	/** Datatable creation policy: enabled flag, datatable name, and optional schema */
	datatableCreationPolicy = $state<{
		enabled: boolean
		datatable: string | undefined
		schema: string | undefined
	}>({ enabled: false, datatable: undefined, schema: undefined })
	pendingNewCode = $state<string | undefined>(undefined)
	apiTools = $state<Tool<any>[]>([])
	aiChatInput = $state<AIChatInput | null>(null)
	/** Cached datatables for app context (fetched asynchronously) */
	cachedDatatables = $state<AppDatatableElement[]>([])

	private confirmationCallbacks = new Map<string, (value: boolean) => void>()
	private userQuestionCallbacks = new Map<string, (choices: string[] | undefined) => void>()
	private appDatatablesRefreshTimeout: ReturnType<typeof setTimeout> | undefined = undefined

	disabledModes: Partial<Record<AIMode, boolean>> = $state({})
	// Set by AI sessions. Enables the session-only preview tools (open_preview /
	// get_preview_status) and their system-prompt guidance in GLOBAL mode; the
	// global side-panel chat leaves it false so those tools aren't offered.
	isSessionChat = false
	// The session this manager belongs to (session chats only). Carried into the
	// tool `helpers` in GLOBAL mode so the preview/deploy tools dispatch to THIS
	// session rather than the UI-active one — keeps backgrounded sessions isolated.
	sessionId: string | undefined = undefined
	// Resolves the workspace this chat operates on. Session chats set it to their
	// own (possibly forked) workspace so the chat targets it WITHOUT switching the
	// global workspaceStore. Undefined for the global side-panel chat, which
	// follows the active workspace. Always read via `operatingWorkspace`.
	workspaceResolver: (() => string | undefined) | undefined = undefined

	// The workspace every workspace-scoped chat action targets — skills, tool
	// loop, logging, user-message context, and commit. Session-resolved when a
	// resolver is set, else the globally-active workspace.
	get operatingWorkspace(): string | undefined {
		return this.workspaceResolver?.() ?? get(workspaceStore)
	}

	// Fired whenever the active chat id changes away from the one the consumer
	// knows (a "/clear" rotation or a history switch). Session runtimes wire this
	// to keep the session record's chatId aligned — the compare-page handoff
	// (`from_session`) reads it, and a stale id would preselect the previous
	// chat's items. Set here (not imported) to avoid a copilot→sessions cycle.
	onChatRotated: ((chatId: string) => void) | undefined = undefined

	// Workspace items the CURRENT chat modified via AI tool calls, as
	// `${UserDraftItemKind}:${storagePath}` keys (see modifiedItemsMask.ts).
	// undefined = untracked: only the global side-panel chat (never initialised),
	// which falls back to the show-all bar. Session chats are always tracked (a
	// SvelteSet, even empty) — see loadPastChat/initRuntime — so their Edits
	// surface never claims drafts the session didn't touch. Reactive so the
	// session bar updates as tools record mid-turn.
	modifiedItems = $state<SvelteSet<string> | undefined>(undefined)

	// Start tracking for a brand-new session chat (empty = "tracked, nothing yet").
	initModifiedItemsTracking() {
		this.modifiedItems = new SvelteSet()
	}

	// Record an item an AI tool call created/edited/deleted. No-op when untracked
	// (the global singleton never initialises the set), so it stays unaffected.
	recordModifiedItem(itemKind: UserDraftItemKind, storagePath: string) {
		this.modifiedItems?.add(maskKey(itemKind, storagePath))
	}

	// Un-record an item whose chat-made change was discarded — without this the
	// still-existing deployed item would keep reading as this chat's "Deployed"
	// edit. Persisted immediately: unlike recordModifiedItem (whose persistence
	// rides on the turn's saveChat), a discard can fire from the review dock
	// outside any turn, and waiting would resurrect the entry on reload.
	async removeModifiedItem(itemKind: UserDraftItemKind, storagePath: string) {
		if (!this.modifiedItems?.delete(maskKey(itemKind, storagePath))) return
		await this.#persistModifiedItems()
	}

	// Move a mask entry to the path a draft actually deployed to. A draft-only
	// flow/app parks at a synthetic `draft_{uuid}` storage path and deploys to
	// its chosen path — without the move, the existence check at the synthetic
	// path fails after reload and the deployed row vanishes from the dock.
	async renameModifiedItem(itemKind: UserDraftItemKind, fromPath: string, toPath: string) {
		if (fromPath === toPath) return
		if (!this.modifiedItems?.delete(maskKey(itemKind, fromPath))) return
		this.modifiedItems.add(maskKey(itemKind, toPath))
		await this.#persistModifiedItems()
	}

	// Serialized, snapshot-at-write-time persistence: two rapid dock actions
	// would otherwise race their saveChat writes, and the earlier (staler)
	// snapshot could land last — dropping the later mutation until the next
	// turn-end save.
	#maskPersistQueue: Promise<void> = Promise.resolve()
	#persistModifiedItems(): Promise<void> {
		this.#maskPersistQueue = this.#maskPersistQueue.then(() =>
			this.historyManager
				.saveChat(
					this.displayMessages,
					this.messages,
					this.contextUsage,
					this.modifiedItems ? [...this.modifiedItems] : undefined
				)
				// Swallow (and log) a failed write so it can't wedge the queue as a
				// rejected link — the next persist snapshots the full current set, so
				// a lost write self-heals on the next mutation or turn-end save.
				.catch((e) => console.error('Failed to persist modified-items mask', e))
		)
		return this.#maskPersistQueue
	}

	// ===== Background jobs (global/sessions chat only) =====
	//
	// A test-run tool that doesn't finish within the inline wait detaches: it
	// returns a "still running" handle to the model and registers the job here.
	// A single poller advances all detached jobs; on completion it fills the tool
	// card and queues a notify-only note for the model's next turn.

	private isJobNonTerminal(status: ChatJobStatus): boolean {
		// suspended (awaiting approval) and scheduled are non-terminal — the poller
		// MUST keep watching them, else an approval would never clear from the tray.
		return (
			status === 'queued' ||
			status === 'running' ||
			status === 'suspended' ||
			status === 'scheduled'
		)
	}

	/** Record a job the moment it starts, so the tray shows it while it is still
	 * inline-waiting. Idempotent on jobId. The init carries the serializable
	 * `resultFormat` (persisted), so completion formatting survives a reload. */
	registerJob = (init: ChatJobInit) => {
		if (this.backgroundJobs.some((j) => j.jobId === init.jobId)) return
		this.backgroundJobs = [
			...this.backgroundJobs,
			{ ...init, createdAt: Date.now(), status: 'queued', detached: false, reported: false }
		]
	}

	/** Merge a partial update into a tracked job by id. */
	updateJob = (jobId: string, update: Partial<ChatJob>) => {
		const idx = this.backgroundJobs.findIndex((j) => j.jobId === jobId)
		if (idx === -1) return
		this.backgroundJobs[idx] = { ...this.backgroundJobs[idx], ...update }
		this.backgroundJobs = [...this.backgroundJobs]
	}

	/** A job left the inline wait — hand it to the background poller. */
	markJobDetached = (jobId: string) => {
		this.updateJob(jobId, { detached: true })
		this.#ensureJobPoller()
		void this.#persistBackgroundJobs()
	}

	/** User-facing cancel from the jobs tray. */
	cancelJob = async (jobId: string) => {
		const job = this.backgroundJobs.find((j) => j.jobId === jobId)
		if (!job) return
		try {
			await JobService.cancelQueuedJob({ workspace: job.workspace, id: jobId, requestBody: {} })
			// Don't mark terminal here: a bare `status: 'canceled'` would (a) leave the
			// `job` snapshot that drives JobStatusIcon stale (badge stuck on running)
			// and (b) make isJobNonTerminal false so the poller stops before it can
			// refresh either. Let the poller observe the canceled CompletedJob and set
			// status + job together; poke it so the tray converges within a tick.
			this.refreshBackgroundJobs()
		} catch (e) {
			console.error('Failed to cancel job', jobId, e)
			sendUserToast('Failed to cancel job', true)
		}
	}

	/** Remove a finished job from the tray. */
	dismissJob = (jobId: string) => {
		this.backgroundJobs = this.backgroundJobs.filter((j) => j.jobId !== jobId)
		void this.#persistBackgroundJobs()
	}

	/** Force an immediate background-job poll (e.g. right after an approval) instead
	 * of waiting for the next scheduled tick. */
	refreshBackgroundJobs = () => {
		this.#stopJobPoller()
		this.#jobPollDelay = 2000
		void this.#pollBackgroundJobs()
	}

	#ensureJobPoller() {
		if (this.#jobPollTimer !== undefined) return
		// A poll pass is running (it cleared #jobPollTimer on entry). It reschedules
		// from the current job set when it finishes, so the job that just detached is
		// already covered. Scheduling here instead would create a second timer that the
		// end-of-pass reschedule overwrites WITHOUT clearing — orphaning it into a
		// duplicate, self-perpetuating poll chain. Coalesce into the active pass.
		if (this.#isPolling) return
		if (!this.backgroundJobs.some((j) => j.detached && this.isJobNonTerminal(j.status))) return
		this.#jobPollDelay = 2000
		this.#scheduleJobPoll()
	}

	#scheduleJobPoll() {
		this.#jobPollTimer = setTimeout(() => void this.#pollBackgroundJobs(), this.#jobPollDelay)
	}

	#stopJobPoller() {
		if (this.#jobPollTimer !== undefined) {
			clearTimeout(this.#jobPollTimer)
			this.#jobPollTimer = undefined
		}
	}

	// Guarded entry point for every poll trigger (scheduled tick, #ensureJobPoller,
	// and refreshBackgroundJobs): if a pass is already running, coalesce into it
	// instead of starting a second concurrent chain that would double the poll rate.
	async #pollBackgroundJobs() {
		if (this.#isPolling) return
		this.#isPolling = true
		try {
			await this.#runBackgroundJobsPoll()
		} finally {
			this.#isPolling = false
		}
	}

	async #runBackgroundJobsPoll() {
		this.#jobPollTimer = undefined
		const gen = this.#jobPollGeneration
		const pending = this.backgroundJobs.filter((j) => j.detached && this.isJobNonTerminal(j.status))
		if (pending.length === 0) return

		let anyTerminal = false
		for (const job of pending) {
			try {
				const fetched = await JobService.getJob({
					workspace: job.workspace,
					id: job.jobId,
					noLogs: false,
					noCode: true
				})
				// The user switched conversations while this getJob was in flight; its
				// result belongs to a chat that's gone. Drop it rather than mutate the
				// newly-loaded one (which re-armed its own poller on load).
				if (gen !== this.#jobPollGeneration) return
				this.#jobPollFailures.delete(job.jobId)
				if (fetched.type === 'CompletedJob') {
					anyTerminal = true
					this.#onBackgroundJobComplete(job, fetched as CompletedJob)
				} else {
					// Store the derived status and the trimmed Job together so the tray
					// badge (JobStatusIcon) and the scalar status can never drift.
					this.updateJob(job.jobId, {
						status: deriveChatJobStatus(fetched),
						job: trimJob(fetched)
					})
				}
			} catch (e) {
				// Same generation guard as the success path — a switch during the failing
				// getJob means this result is for a conversation that's gone.
				if (gen !== this.#jobPollGeneration) return
				// A vanished job (404) or repeated failures must not keep the poller
				// alive forever — now that suspended/scheduled are polled too, drain it
				// as failed so isJobNonTerminal lets the poller stop.
				const httpStatus = (e as { status?: number })?.status
				const failures = (this.#jobPollFailures.get(job.jobId) ?? 0) + 1
				this.#jobPollFailures.set(job.jobId, failures)
				if (httpStatus === 404 || failures >= 5) {
					this.#jobPollFailures.delete(job.jobId)
					// Vanished (404) or unreachable after repeated polls. Mark it failed WITH
					// a snapshot + tool-card patch (mirroring #onBackgroundJobComplete) so
					// neither the tray badge nor the launching tool card stays frozen on
					// "running" — a bare `status: 'failure'` with no `job` would render the
					// orange queued badge (JobsSegment's `!job.job` fallback). The synthetic
					// failed CompletedJob keeps the `success`-key discriminant so JobStatusIcon
					// and deriveChatJobStatus agree. No model note/auto-resume: a vanished job
					// isn't a meaningful completion to react to (usually transient infra).
					const gone = {
						type: 'CompletedJob',
						id: job.jobId,
						success: false,
						canceled: false
					} as unknown as CompletedJob
					this.updateJob(job.jobId, { status: 'failure', reported: true, job: trimJob(gone) })
					this.applyToolStatus(job.toolCallId, {
						content: 'Background job could not be retrieved (it may have been removed)',
						error: `Job ${job.jobId} was unreachable`
					})
					anyTerminal = true
				} else {
					console.error('Failed to poll background job', job.jobId, e)
				}
			}
		}

		if (anyTerminal) {
			void this.#persistBackgroundJobs()
		}

		// Reschedule while anything is still in flight, backing off up to 5s.
		if (this.backgroundJobs.some((j) => j.detached && this.isJobNonTerminal(j.status))) {
			this.#jobPollDelay = Math.min(this.#jobPollDelay + 1000, 5000)
			this.#scheduleJobPoll()
		}

		// Something finished this cycle — if the chat is idle, react to it now
		// instead of waiting for the user's next message. Fire-and-forget so the
		// poller loop above isn't blocked by the turn.
		if (anyTerminal) void this.#maybeAutoResumeFromJobs()
	}

	#onBackgroundJobComplete(job: ChatJob, completed: CompletedJob) {
		const status = deriveChatJobStatus(completed)
		this.updateJob(job.jobId, {
			status,
			durationMs: completed.duration_ms,
			reported: true,
			job: trimJob(completed)
		})
		// If the launching tool stamped a resultFormat, reconstruct its shaped card +
		// model text so the detached path reports the same contract the inline path
		// would (row-capped rows, friendly datatable errors) — even after a reload,
		// since resultFormat is persisted on the job. A canceled job skips formatting:
		// its card is the neutral "canceled" state, not a result.
		const formatted =
			status === 'canceled' || !job.resultFormat
				? undefined
				: formatChatJobCompletion(completed, job.resultFormat)
		// Fill the tool card that launched it (we run outside a turn here).
		this.applyToolStatus(job.toolCallId, formatted?.card ?? completedJobToolStatus(completed))
		// A user-canceled job needs no model note or auto-resume: the user stopped it
		// deliberately, so announcing it (as "FAILED", since a canceled job isn't a
		// success) or burning a turn on it would be noise.
		if (status === 'canceled') return
		// Queue a completion note for the model. Delivered on the next turn —
		// either the user's next message or an idle auto-resume (fired by the poller).
		this.pendingJobNotes = [
			...this.pendingJobNotes,
			backgroundJobCompletionNote(job.jobId, job.label, completed, formatted?.llmText)
		]
	}

	/**
	 * Stage 2 wake: when a background job finishes and the chat is otherwise idle,
	 * start a turn on the user's behalf so the model reacts to the result (reports
	 * it, continues the plan) instead of waiting for the next manual message. The
	 * rich completion note reaches the model via the pendingJobNotes preamble in
	 * sendRequest; the visible bubble is just a short, clearly-automated line.
	 *
	 * Bounded so it can't run away: fires only when idle (no in-flight turn) and
	 * only when notes exist — and sendRequest drains the notes, so a turn that
	 * doesn't spawn a new job leaves nothing to re-trigger on. A turn that DOES
	 * spawn another job resumes again when that one finishes, which is the point.
	 */
	async #maybeAutoResumeFromJobs() {
		if (this.#autoResuming) return
		// Global/sessions chat only (the only mode with a jobs tray + preamble).
		if (this.mode !== AIMode.GLOBAL) return
		// Mid-turn: the notes will ride that turn's preamble, so don't start another.
		if (this.loading) return
		if (this.pendingJobNotes.length === 0) return
		// Nothing to continue (empty chat), or the user is mid-compose — don't
		// clobber their draft or auto-send it. Their eventual send carries the notes.
		if (this.messages.length === 0 || this.instructions.trim()) return
		this.#autoResuming = true
		try {
			const count = this.pendingJobNotes.length
			this.instructions =
				count === 1 ? 'A background job just finished.' : `${count} background jobs just finished.`
			await this.sendRequest()
		} catch (e) {
			console.error('Auto-resume after background job failed', e)
		} finally {
			this.#autoResuming = false
		}
	}

	// Serialized snapshot-at-write persistence, mirroring #persistModifiedItems.
	// Omits the modified-items mask so a concurrent mask write isn't clobbered
	// (saveChat keeps the prior mask when it is undefined).
	#jobPersistQueue: Promise<void> = Promise.resolve()
	#persistBackgroundJobs(): Promise<void> {
		this.#jobPersistQueue = this.#jobPersistQueue.then(() =>
			this.historyManager
				.saveChat(
					this.displayMessages,
					this.messages,
					this.contextUsage,
					undefined,
					$state.snapshot(this.backgroundJobs)
				)
				.catch((e) => console.error('Failed to persist background jobs', e))
		)
		return this.#jobPersistQueue
	}

	/** Reset background-job state on conversation switch. */
	private clearBackgroundJobs() {
		this.#stopJobPoller()
		// Invalidate any in-flight poll so its post-await continuation can't write
		// into the conversation we're switching to.
		this.#jobPollGeneration++
		this.backgroundJobs = []
		this.pendingJobNotes = []
	}

	/** Merge a status patch into the tool card identified by tool_call_id, or
	 * create it. Shared by the per-turn setToolStatus callback and the background
	 * job poller (which runs outside a turn). */
	applyToolStatus = (id: string, metadata?: Partial<ToolDisplayMessage>) => {
		const existingIdx = this.displayMessages.findIndex(
			(m) => m.role === 'tool' && m.tool_call_id === id
		)
		if (existingIdx !== -1) {
			const existing = this.displayMessages[existingIdx] as ToolDisplayMessage
			if (existing.content.length === 0 && metadata?.error) {
				this.displayMessages[existingIdx].content = metadata.error
			}
			this.displayMessages[existingIdx] = {
				...existing,
				...(metadata || {})
			} as ToolDisplayMessage
		} else {
			const newMessage: ToolDisplayMessage = {
				role: 'tool',
				tool_call_id: id,
				content: metadata?.content ?? metadata?.error ?? '',
				...(metadata || {})
			}
			this.displayMessages.push(newMessage)
		}
	}

	// Workspace AI skills (name + description) advertised in the GLOBAL system
	// prompt and surfaced as slash commands in session chat. Loaded
	// asynchronously when entering GLOBAL mode; the system message is rebuilt
	// once they resolve.
	globalSkills = $state<AiSkillListItem[]>([])
	private globalSkillsRefreshId = 0

	// Built-in session-chat slash commands, listed in the command picker
	// alongside workspace skills. Unlike a skill, these run locally and never
	// reach the model; the submit path intercepts them first, so they shadow any
	// workspace skill of the same name.
	readonly sessionBuiltinCommands: AiSkillListItem[] = [
		{ name: COMPACT_COMMAND_NAME, description: 'Summarize the conversation to free up context' },
		{ name: CLEAR_COMMAND_NAME, description: 'Clear the conversation and start a new chat' }
	]

	// Built-ins followed by workspace skills, with any skill whose name collides
	// with a built-in dropped: the picker keys leaves by name, so a duplicate
	// would break its keyed list and ambiguous-resolve nav. Built-ins win — they
	// already shadow same-named skills at execution (the submit interception).
	sessionCommands: AiSkillListItem[] = $derived([
		...this.sessionBuiltinCommands,
		...this.globalSkills.filter((s) => !this.sessionBuiltinCommands.some((b) => b.name === s.name))
	])

	allowedModes: Record<AIMode, boolean> = $derived({
		script:
			this.flowAiChatHelpers === undefined &&
			this.scriptEditorOptions !== undefined &&
			!this.disabledModes.script,
		flow: this.flowAiChatHelpers !== undefined && !this.disabledModes.flow,
		app: this.appAiChatHelpers !== undefined && !this.disabledModes.app,
		navigator: !this.disabledModes.navigator,
		ask: !this.disabledModes.ask,
		API: !this.disabledModes.API,
		// Dev-only gate. See `./global/gate.ts` for how to enable.
		global: isAIModeVisible(AIMode.GLOBAL)
	})

	open = $derived(chatState.size > 0)

	// one token is ~ 4 characters
	private estimateMessagesTokens = (messages: ChatCompletionMessageParam[]) => {
		return messages.reduce((acc, message) => {
			const tokenPerCharacter = 4
			if (typeof message.content === 'string') {
				acc += message.content.length / tokenPerCharacter
			} else if (message.content) {
				acc += JSON.stringify(message.content).length / tokenPerCharacter
			}
			if (message.role === 'assistant' && message.tool_calls) {
				acc += JSON.stringify(message.tool_calls).length / tokenPerCharacter
			}
			return acc
		}, 0)
	}

	/** Estimated tokens of the parts the messages array doesn't carry: the
	 * current system prompt and tool definitions. */
	private estimateOverheadTokens = () => {
		const tokenPerCharacter = 4
		const systemTokens =
			typeof this.systemMessage.content === 'string'
				? this.systemMessage.content.length / tokenPerCharacter
				: 0
		const toolTokens =
			this.tools.length > 0
				? JSON.stringify(this.tools.map((t) => t.def)).length / tokenPerCharacter
				: 0
		return systemTokens + toolTokens
	}

	/**
	 * chars/4 estimate of the full context as currently stored: messages plus
	 * the system prompt and tool definitions the next request would carry.
	 * Recomputed from scratch at each read — never accumulated — so errors
	 * don't compound.
	 */
	private estimateWholeContextTokens = () =>
		Math.round(this.estimateMessagesTokens(this.messages) + this.estimateOverheadTokens())

	/**
	 * How full the context is right now — the single fallback rule, shared by
	 * the compaction trigger and the usage indicator: the provider's exact
	 * report when one describes the current history, a fresh estimate
	 * otherwise. Estimating at the read point (rather than writing estimates
	 * into `contextUsage`) means no code path that mutates history can leave
	 * a stale or missing value behind.
	 */
	contextTokens = $derived.by(() => this.contextUsage ?? this.estimateWholeContextTokens())

	/**
	 * Drop-oldest compaction. Deletes messages from the front of the STORED
	 * history (the API messages — displayMessages keep the full conversation
	 * for the user) until at least `tokensToFree` estimated tokens are freed
	 * AND the remaining history starts on a user message: a leading tool
	 * result or assistant turn would dangle without the messages that
	 * introduced it. The most recent user message is never dropped. Returns
	 * the estimated tokens freed.
	 */
	compactOldestMessages = (tokensToFree: number): number => {
		const last = this.messages.length - 1
		let drop = 0
		let freed = 0
		while (drop < last) {
			if (freed >= tokensToFree && this.messages[drop].role === 'user') {
				break
			}
			freed += this.estimateMessagesTokens([this.messages[drop]])
			drop++
		}
		if (drop === 0) {
			return 0
		}
		this.messages = this.messages.slice(drop)
		// User display messages carry the index of their API message so restart
		// can rewind to it; re-base them on the compacted history. A message
		// whose API counterpart was dropped clamps to 0: everything before it
		// was dropped too (compaction only removes prefixes), so restarting
		// from it restarts from an empty history.
		this.displayMessages = this.displayMessages.map((m) =>
			m.role === 'user' ? { ...m, index: Math.max(0, m.index - drop) } : m
		)
		return freed
	}

	/**
	 * Core summarize + rewrite, shared by automatic and manual compaction. Sends
	 * the prefix to the summarizer, then replaces the summarized prefix with a
	 * single summary message in `messages` (as a user message) and
	 * `displayMessages` (as a `summary` boundary). Surviving tail user messages
	 * have their restart `index` re-based onto the new history: the summary
	 * occupies slot 0, so a tail user message that was at `keepFrom` lands at slot
	 * 1. `displayKeepFrom` is where the kept tail begins in `displayMessages`.
	 *
	 * Owns only the `compacting` flag and the history rewrite; callers own trigger
	 * policy (circuit breaker, gates) and persistence. Returns the outcome —
	 * 'aborted' is a user Stop (history left untouched), distinct from 'error'.
	 */
	private runSummarization = async (
		prefix: ChatCompletionMessageParam[],
		tail: ChatCompletionMessageParam[],
		keepFrom: number,
		displayKeepFrom: number,
		abortController: AbortController
	): Promise<'ok' | 'empty' | 'aborted' | 'error'> => {
		this.compacting = true
		try {
			const raw = await getNonStreamingCompletion(
				[
					...sanitizeToolCallArguments(prefix),
					{ role: 'user', content: getCompactionSummaryPrompt() }
				],
				abortController
			)
			const formatted = formatCompactSummary(raw ?? '')
			if (!formatted) {
				return 'empty'
			}

			this.messages = [{ role: 'user', content: buildSummaryMessageContent(formatted) }, ...tail]

			// Replace the summarized display prefix with the boundary marker and
			// re-base the surviving tail's restart indices (the summary occupies
			// slot 0, so the tail now starts at slot 1).
			this.displayMessages = [
				{ role: 'summary', content: formatted },
				...this.displayMessages
					.slice(displayKeepFrom)
					.map((m) => (m.role === 'user' ? { ...m, index: m.index - keepFrom + 1 } : m))
			]

			// The provider report described the pre-compaction history; the new
			// history is much smaller, so clear it and let readers re-estimate.
			this.contextUsage = undefined
			return 'ok'
		} catch (err) {
			if (abortController.signal.aborted) {
				return 'aborted'
			}
			console.error('Conversation summarization failed', err)
			return 'error'
		} finally {
			this.compacting = false
		}
	}

	/**
	 * Summary-based partial compaction. Summarizes the older PREFIX of the stored
	 * history into a single user message and keeps the recent tail verbatim,
	 * bringing the history down to roughly the target ratio while preserving the
	 * intent, decisions, and recent work that drop-oldest would discard.
	 *
	 * The tail grows from the most recent message until it fills `tailBudget`,
	 * then snaps forward to a user-message boundary (a leading tool/assistant
	 * message would dangle without the turn that introduced it). The summary
	 * replaces the prefix in BOTH `messages` (as a user message) and
	 * `displayMessages` (as a `summary` boundary); surviving tail user messages
	 * have their restart `index` re-based onto the new history.
	 *
	 * Returns true on success. Returns false — caller falls back to drop-oldest —
	 * when summarization isn't worthwhile or fails (user abort, empty summary, or
	 * the circuit breaker being tripped).
	 */
	private summarizeAndCompact = async (contextWindow: number): Promise<boolean> => {
		if (this.consecutiveCompactionFailures >= MAX_CONSECUTIVE_COMPACTION_FAILURES) {
			return false
		}
		const abortController = this.abortController
		if (!abortController) {
			return false
		}

		const tailBudget =
			contextWindow * COMPACTION_TARGET_RATIO -
			SUMMARY_OUTPUT_RESERVE_TOKENS -
			this.estimateOverheadTokens()
		if (tailBudget <= 0) {
			return false
		}

		const last = this.messages.length - 1
		if (last < 1) {
			return false
		}

		// Grow the tail from the most recent message downward while it fits the
		// budget; always keep at least the last message.
		let keepFrom = last
		let tailTokens = 0
		for (let i = last; i >= 1; i--) {
			const t = this.estimateMessagesTokens([this.messages[i]])
			if (i < last && tailTokens + t > tailBudget) {
				break
			}
			tailTokens += t
			keepFrom = i
		}
		// The tail must start on a user message — move the boundary forward over
		// any leading tool/assistant messages, folding them into the prefix.
		while (keepFrom < last && this.messages[keepFrom].role !== 'user') {
			keepFrom++
		}

		const prefix = this.messages.slice(0, keepFrom)
		const tail = this.messages.slice(keepFrom)
		if (prefix.length < MIN_PREFIX_MESSAGES_TO_SUMMARIZE || tail.length === 0) {
			return false
		}

		// The user message at the boundary has a display counterpart with the same
		// index; resolve it before any mutation so a corrupt transcript can never
		// result from an unexpected miss.
		const displayKeepFrom = this.displayMessages.findIndex(
			(m) => m.role === 'user' && m.index >= keepFrom
		)
		if (displayKeepFrom === -1) {
			this.consecutiveCompactionFailures++
			return false
		}

		const result = await this.runSummarization(
			prefix,
			tail,
			keepFrom,
			displayKeepFrom,
			abortController
		)
		if (result === 'ok') {
			this.consecutiveCompactionFailures = 0
			return true
		}
		// 'aborted' is a user Stop during the in-flight summary — a turn cancel, not
		// a compaction failure, so it doesn't count toward the circuit breaker.
		if (result === 'empty' || result === 'error') {
			this.consecutiveCompactionFailures++
		}
		return false
	}

	/**
	 * Manual compaction (the `/compact` session command): summarize the ENTIRE
	 * stored history into a single summary message and keep nothing verbatim, so
	 * the next message continues from the summary alone. Unlike the automatic
	 * trigger it ignores the context-window budget, the circuit breaker, and the
	 * prefix-size gate — the user asked for it explicitly — and runs on its own
	 * abort controller so the Stop button (`cancel`) can interrupt the in-flight
	 * summary, leaving history untouched.
	 */
	compactManually = async (): Promise<void> => {
		if (this.loading) {
			return
		}
		// A summary round-trip only pays off once there's a prior exchange to fold
		// in; a single message (or none) has nothing to compact.
		if (this.messages.length < 2) {
			sendUserToast('Nothing to compact yet.')
			return
		}

		const abortController = new AbortController()
		this.abortController = abortController
		this.loading = true
		let result: 'ok' | 'empty' | 'aborted' | 'error' = 'error'
		try {
			// Everything is the prefix, nothing is kept verbatim: keepFrom and
			// displayKeepFrom point past the end so the kept tail is empty.
			result = await this.runSummarization(
				[...this.messages],
				[],
				this.messages.length,
				this.displayMessages.length,
				abortController
			)
			switch (result) {
				case 'ok':
					await this.historyManager.saveChat(
						this.displayMessages,
						this.messages,
						this.contextUsage,
						this.modifiedItems ? [...this.modifiedItems] : undefined
					)
					sendUserToast('Conversation compacted.')
					break
				case 'empty':
					sendUserToast('Compaction produced an empty summary — conversation left unchanged.', true)
					break
				case 'error':
					sendUserToast('Failed to compact the conversation.', true)
					break
				// 'aborted' (user Stop): history untouched, no toast.
			}
		} finally {
			this.loading = false
		}

		// Flush a message typed while compaction ran. Mirrors the send-turn
		// epilogue (loading gated its capture): auto-send after a successful
		// compaction or a deliberate user cancel — the user is ready to move on —
		// while a failed/empty compaction or a programmatic cancel leaves it queued.
		if ((result === 'ok' || this.wasCancelledByUser()) && this.queuedMessage) {
			const next = this.queuedMessage
			this.queuedMessage = ''
			const accepted = await this.sendRequest({ instructions: next })
			if (accepted === false) {
				this.queuedMessage = next
			}
		}
	}

	loadApiTools = async () => {
		try {
			this.apiTools = await loadApiTools()
			if (this.mode === AIMode.API) {
				this.tools = [searchDocsTool, readDocsPageTool, ...this.apiTools]
			}
		} catch (err) {
			console.error('Error loading api tools', err)
			this.apiTools = []
		}
	}

	// Request confirmation from user for a tool call
	requestConfirmation = (toolId: string): Promise<boolean> => {
		if (this.autoAcceptToolConfirmationsActive) {
			return Promise.resolve(true)
		}

		return new Promise((resolve) => {
			this.confirmationCallbacks.set(toolId, resolve)
		})
	}

	// Handle confirmation response for a specific tool
	handleToolConfirmation = (toolId: string, confirmed: boolean) => {
		const confirmationCallback = this.confirmationCallbacks.get(toolId)
		if (confirmationCallback) {
			confirmationCallback(confirmed)
			this.confirmationCallbacks.delete(toolId)
		}
	}

	private acceptPendingToolConfirmations = () => {
		for (const confirmationCallback of this.confirmationCallbacks.values()) {
			confirmationCallback(true)
		}
		this.confirmationCallbacks.clear()
	}

	private acceptPendingFlowEdits = (flowHelpers = this.flowAiChatHelpers) => {
		if (flowHelpers?.hasPendingChanges()) {
			flowHelpers.acceptAllModuleActions()
		}
	}

	setAutonomyMode = (mode: AIAutonomyMode) => {
		this.autonomyMode = mode
		persistAutonomyMode(mode)

		if (this.autoAcceptToolConfirmationsActive) {
			this.acceptPendingToolConfirmations()
		}
		if (this.autoAcceptEditsActive) {
			this.acceptPendingFlowEdits()
		}
	}

	setAutoAcceptToolConfirmations = (enabled: boolean) => {
		this.setAutonomyMode(enabled ? AIAutonomyMode.YOLO : AIAutonomyMode.DEFAULT)
	}

	// Re-read the autonomy mode from the user-scoped key when the logged-in
	// email resolves or changes. Claims legacy un-namespaced keys on first
	// login; falls back to the safe default when logged out so we never leave a
	// prior user's YOLO mode active. Registered only for the module-level
	// singleton (constructed before the email is known) — per-session managers
	// are constructed post-login and read the scoped value directly.
	hydrateUserScopedAutonomy = () => {
		migrateLegacyAutonomyKeys()
		this.autonomyMode = getPersistedAutonomyMode()
	}

	applyScriptEditorCode = async (code: string, opts?: ReviewChangesOpts) => {
		if (this.autoAcceptEditsActive && opts?.mode === 'revert') {
			return
		}

		const effectiveOpts =
			this.autoAcceptEditsActive && (opts?.mode ?? 'apply') === 'apply'
				? ({ ...opts, mode: 'apply', applyAll: true } satisfies ReviewChangesOpts)
				: opts
		await this.scriptEditorApplyCode?.(code, effectiveOpts)
	}

	requestUserQuestion = (
		toolId: string,
		_question: UserQuestionDisplay
	): Promise<string[] | undefined> => {
		return new Promise((resolve) => {
			this.userQuestionCallbacks.set(toolId, resolve)
		})
	}

	handleUserQuestionAnswer = (toolId: string, choices: string[]) => {
		const callback = this.userQuestionCallbacks.get(toolId)
		if (!callback) {
			return
		}

		// Display-only readback for the collapsed tool-header: a compact comma list.
		// The model-facing return (bare string / newline-bulleted) is built by the
		// tool fn from the resolved choices below.
		const answerSummary = choices.join(', ')

		this.displayMessages = this.displayMessages.map((message) => {
			if (message.role === 'tool' && message.tool_call_id === toolId && message.userQuestion) {
				return {
					...message,
					content: `User answered question: ${answerSummary}`,
					isLoading: false,
					userQuestion: {
						...message.userQuestion,
						selectedChoices: choices
					}
				}
			}
			return message
		})

		callback(choices)
		this.userQuestionCallbacks.delete(toolId)
	}

	setAiChatInput(aiChatInput: AIChatInput | null) {
		this.aiChatInput = aiChatInput
	}

	/** Queue the message typed while a turn is streaming. There is only ever
	 * one queued message; pressing Enter again appends the new text as another
	 * line so it all goes out as a single message. */
	queueMessage(text: string) {
		const trimmed = text.trim()
		if (!trimmed) {
			return
		}
		this.queuedMessage = this.queuedMessage ? `${this.queuedMessage}\n${trimmed}` : trimmed
	}

	/** Remove the queued message and put its text back into the input. */
	dequeueMessage() {
		if (!this.queuedMessage) {
			return
		}
		const message = this.queuedMessage
		this.queuedMessage = ''
		this.restoreToInput(message)
	}

	/** Put text the user typed back where they can see it: into the input
	 * when it's mounted, otherwise back into the queue so it reappears with
	 * the chat panel instead of being silently dropped. */
	private restoreToInput(text: string) {
		if (this.aiChatInput) {
			this.aiChatInput.prependText(text)
		} else {
			this.queuedMessage = text
		}
	}

	focusInput() {
		if (this.aiChatInput) {
			this.aiChatInput.focusInput()
		}
	}

	updateMode(currentMode: AIMode) {
		if (
			!this.allowedModes[currentMode] &&
			Object.keys(this.allowedModes).filter((k) => this.allowedModes[k]).length === 1
		) {
			const firstKey = Object.keys(this.allowedModes).filter((k) => this.allowedModes[k])[0]
			this.changeMode(firstKey as AIMode)
		}
	}

	private getScriptWorkspaceMutationTarget = (): WorkspaceMutationTarget => {
		const path = this.scriptEditorOptions?.path
		const workspacePath = isWorkspacePath(path) ? path : undefined
		return {
			kind: 'script',
			path: workspacePath,
			deployed:
				workspacePath !== undefined && this.scriptEditorOptions?.lastDeployedCode !== undefined
		}
	}

	private getFlowWorkspaceMutationTarget = (): WorkspaceMutationTarget => {
		return {
			kind: 'flow',
			path: this.flowOptions?.path,
			deployed:
				!!this.flowOptions?.path &&
				!!this.flowOptions.lastDeployedFlow &&
				!this.flowOptions.lastDeployedFlow.draft_only
		}
	}

	changeMode(
		mode: AIMode,
		pendingPrompt?: string,
		options?: {
			closeScriptSettings?: boolean
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
			workflowAsCode?: boolean
		}
	) {
		if (!isAIModeVisible(mode)) return
		if (mode === AIMode.SCRIPT && !tryGetCurrentModel()) return
		this.mode = mode
		this.pendingPrompt = pendingPrompt ?? ''
		if (mode === AIMode.SCRIPT) {
			const currentModel = getCurrentModel()
			const customPrompt = getCombinedCustomPrompt(mode)
			const lang = options?.lang ?? this.scriptEditorOptions?.lang ?? 'bun'
			const workflowAsCode =
				options?.workflowAsCode ??
				(options?.lang ? false : (this.scriptEditorOptions?.workflowAsCode ?? false))
			const context = this.contextManager.getSelectedContext()
			this.systemMessage = prepareScriptSystemMessage(
				currentModel,
				lang,
				{ isPreprocessor: options?.isPreprocessor, workflowAsCode },
				customPrompt
			)
			this.systemMessage.content = this.systemMessage.content
			this.tools = [...prepareScriptTools(currentModel, lang, context)]
			this.helpers = {
				getScriptOptions: () => {
					return {
						code: this.scriptEditorOptions?.getCode() ?? '',
						lang: lang,
						path: this.scriptEditorOptions?.path ?? '',
						args: this.scriptEditorOptions?.args ?? {}
					}
				},
				getWorkspaceMutationTarget: this.getScriptWorkspaceMutationTarget,
				applyCode: (code: string, opts?: ReviewChangesOpts) => {
					return this.applyScriptEditorCode(code, opts)
				},
				getLintErrors: () => {
					if (this.scriptEditorGetLintErrors) {
						return this.scriptEditorGetLintErrors()
					}
					return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
				}
			}
			if (options?.closeScriptSettings) {
				const closeComponent = triggerablesByAi['close-script-builder-settings']
				if (closeComponent) {
					closeComponent.onTrigger?.()
				}
			}
		} else if (mode === AIMode.FLOW) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareFlowSystemMessage(customPrompt)
			this.systemMessage.content = this.systemMessage.content
			this.tools = [...flowTools]
			this.helpers = {
				...(this.flowAiChatHelpers ?? {}),
				getWorkspaceMutationTarget: this.getFlowWorkspaceMutationTarget
			}
		} else if (mode === AIMode.NAVIGATOR) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareNavigatorSystemMessage(customPrompt)
			this.tools = [this.changeModeTool, ...navigatorTools]
			this.helpers = {}
		} else if (mode === AIMode.ASK) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareAskSystemMessage(customPrompt)
			this.tools = [...askTools]
			this.helpers = {}
		} else if (mode === AIMode.API) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareApiSystemMessage(customPrompt)
			this.tools = [searchDocsTool, readDocsPageTool, ...this.apiTools]
			this.helpers = {}
		} else if (mode === AIMode.GLOBAL) {
			this.configureGlobalMode()
			void this.refreshGlobalSkills()
		} else if (mode === AIMode.APP) {
			const customPrompt = getCombinedCustomPrompt(mode)
			this.systemMessage = prepareAppSystemMessage(customPrompt)
			this.tools = [...getAppTools()]
			this.helpers = this.appAiChatHelpers
		}
	}

	// Fetch the workspace's AI skills and, if GLOBAL mode is still active, rebuild
	// the system message so the next chat-loop iteration advertises them. Ignore
	// stale resolves so workspace changes cannot overwrite newer skills.
	// Build the global-mode system message, tools, and helpers, layering on the
	// pipeline surface when a /pipeline editor has registered helpers. Centralized
	// so changeMode, refreshGlobalSkills, and setPipelineHelpers stay consistent —
	// each rebuild would otherwise drop the pipeline augmentation the others added.
	private configureGlobalMode = () => {
		const systemMessage = prepareGlobalSystemMessage(getCustomPromptParts(AIMode.GLOBAL), {
			previewTools: this.isSessionChat,
			skills: this.globalSkills
		})
		const baseHelpers: GlobalToolHelpers = {
			// A session targets its own fixed (possibly forked) workspace, so capture it for
			// permission gating. The global side-panel chat follows the live navigation
			// workspace instead, so leave it unset there — allowedOpenPages reads the store.
			...(this.isSessionChat
				? { sessionId: this.sessionId, operatingWorkspace: this.operatingWorkspace }
				: {}),
			testActiveFlow: async (args?: Record<string, any>) => this.flowAiChatHelpers?.testFlow(args),
			attachedFiles: this.attachedFiles,
			getUserInstructions: () => getUserCustomPrompts()[AIMode.GLOBAL] ?? '',
			setUserInstructions: (instructions: string) => {
				const prompts = getUserCustomPrompts()
				if (instructions.trim()) {
					prompts[AIMode.GLOBAL] = instructions
				} else {
					delete prompts[AIMode.GLOBAL]
				}
				setUserCustomPrompts(prompts)
				this.rebuildGlobalSystemMessage()
			}
		}
		const pipeline = this.pipelineAiChatHelpers
		if (pipeline) {
			systemMessage.content += getPipelinePromptSection(pipeline.getPipelineContext())
			this.tools = [...globalToolsFor({ sessionPreview: this.isSessionChat }), ...pipelineTools]
			this.helpers = { ...baseHelpers, pipeline }
		} else {
			this.tools = globalToolsFor({ sessionPreview: this.isSessionChat })
			this.helpers = baseHelpers
		}
		this.systemMessage = systemMessage
	}

	refreshGlobalSkills = async (workspace = this.operatingWorkspace ?? '') => {
		const refreshId = ++this.globalSkillsRefreshId
		const skills = await loadWorkspaceSkills(workspace)
		if (refreshId !== this.globalSkillsRefreshId) {
			return
		}
		this.globalSkills = skills
		if (this.mode === AIMode.GLOBAL) {
			this.configureGlobalMode()
		}
	}

	// Rebuild the GLOBAL system message in place so an updated user instruction (persisted by
	// the update_user_instructions tool) is picked up on the next chat-loop iteration, which
	// re-reads this.systemMessage via a getter.
	rebuildGlobalSystemMessage = () => {
		if (this.mode !== AIMode.GLOBAL) {
			return
		}
		const systemMessage = prepareGlobalSystemMessage(getCustomPromptParts(AIMode.GLOBAL), {
			previewTools: this.isSessionChat,
			skills: this.globalSkills
		})
		// Preserve the active pipeline-editor augmentation that configureGlobalMode
		// adds — otherwise update_user_instructions (which calls this) would drop the
		// /pipeline/<folder> context + direct-draft/materialize guidance mid-session.
		const pipeline = this.pipelineAiChatHelpers
		if (pipeline) {
			systemMessage.content += getPipelinePromptSection(pipeline.getPipelineContext())
		}
		this.systemMessage = systemMessage
	}

	private expandGlobalSkillCommand = (instructions: string): string => {
		if (!this.isSessionChat || this.mode !== AIMode.GLOBAL || !instructions.startsWith('/')) {
			return instructions
		}
		const match = /^\/([a-z0-9-]+)(?:\s+([\s\S]*))?$/.exec(instructions)
		if (!match) {
			return instructions
		}
		const skill = this.globalSkills.find((s) => s.name === match[1])
		if (!skill) {
			return instructions
		}
		const rest = match[2]?.trim()
		return rest ? `Use the "${skill.name}" skill. ${rest}` : `Use the "${skill.name}" skill.`
	}

	canApplyCode = $derived(this.allowedModes.script && this.mode === AIMode.SCRIPT)

	private changeModeTool = {
		def: {
			type: 'function' as const,
			function: {
				name: 'change_mode',
				description:
					'Change the AI mode to the one specified. Script mode is used to create scripts. Flow mode is used to create flows.' +
					(isGlobalAiEnabled()
						? ' Global mode is used to inspect workspace scripts and flows and create draft changes.'
						: '') +
					' Navigator mode is used to navigate the application and help the user find what they are looking for. API mode is used to make API calls to the Windmill backend.',
				parameters: {
					type: 'object',
					properties: {
						mode: {
							type: 'string',
							description: 'The mode to change to',
							enum: [
								'script',
								'flow',
								...(isGlobalAiEnabled() ? ['global'] : []),
								'navigator',
								'API'
							]
						},
						pendingPrompt: {
							type: 'string',
							description: 'The prompt to send to the new mode to fulfill the user request',
							default: ''
						}
					},
					required: ['mode']
				}
			}
		},
		fn: async ({ args, toolId, toolCallbacks }) => {
			if (!isAIMode(args.mode) || !isAIModeVisible(args.mode)) {
				throw new Error(`AI mode "${args.mode}" is not enabled`)
			}
			toolCallbacks.setToolStatus(toolId, { content: 'Switching to ' + args.mode + ' mode...' })
			this.changeMode(args.mode, args.pendingPrompt, {
				closeScriptSettings: true
			})
			toolCallbacks.setToolStatus(toolId, { content: 'Switched to ' + args.mode + ' mode' })
			return 'Mode changed to ' + args.mode
		}
	}

	openChat = () => {
		chatState.size = this.savedSize > 0 ? this.savedSize : DEFAULT_SIZE
		localStorage.setItem('ai-chat-open', 'true')
	}

	closeChat = () => {
		this.savedSize = chatState.size
		chatState.size = 0
		localStorage.setItem('ai-chat-open', 'false')
	}

	toggleOpen = () => {
		if (chatState.size > 0) {
			this.savedSize = chatState.size
		}
		chatState.size = chatState.size === 0 ? (this.savedSize > 0 ? this.savedSize : DEFAULT_SIZE) : 0
		localStorage.setItem('ai-chat-open', chatState.size === 0 ? 'false' : 'true')
	}

	askAi = (
		prompt: string,
		options: { withCode?: boolean; withDiff?: boolean } = {
			withCode: true,
			withDiff: false
		}
	) => {
		if (this.scriptEditorOptions) {
			this.contextManager.setAskAiContext(options)
		}
		this.instructions = prompt
		this.sendRequest({
			removeDiff: options.withDiff,
			addBackCode: options.withCode === false
		})
		if (options.withDiff) {
			this.scriptEditorShowDiffMode?.()
		}
	}

	retryRequest = (messageIndex: number) => {
		const message = this.displayMessages[messageIndex]
		if (message && message.role === 'user') {
			this.restartGeneration(messageIndex)
			message.error = false
		} else {
			throw new Error('No user message found at the specified index')
		}
	}

	private getLastUserMessage = () => {
		for (let i = this.displayMessages.length - 1; i >= 0; i--) {
			const message = this.displayMessages[i]
			if (message.role === 'user') {
				return message
			}
		}
	}

	private flagLastMessageAsError = () => {
		const lastUserMessage = this.getLastUserMessage()
		if (lastUserMessage) {
			lastUserMessage.error = true
		}
	}

	// Commit an interrupted turn's usable output as context for a follow-up:
	// the tool-paired prefix of completed steps (a dangling tool call would
	// make providers reject the next request) plus the partial answer text.
	// A reasoning-only interrupt instead drops its stuck-open bubble.
	private commitInterruptedTurn = (
		collectedMessages: ChatCompletionMessageParam[],
		partialReply: string
	) => {
		const prefix = truncateToToolPairedPrefix(collectedMessages)
		this.messages = [...this.messages, ...prefix]
		// partialReply can be stale — equal to text already committed inside the
		// prefix (see its capture in onMessageEnd) — so only append when new.
		const lastCommittedText = [...prefix]
			.reverse()
			.find(
				(m): m is ChatCompletionMessageParam & { content: string } =>
					m.role === 'assistant' && typeof m.content === 'string' && !!m.content.trim()
			)?.content
		if (partialReply.trim() && partialReply !== lastCommittedText) {
			this.messages = [...this.messages, { role: 'assistant', content: partialReply }]
		} else {
			const last = this.displayMessages[this.displayMessages.length - 1]
			if (last?.role === 'assistant' && !last.content.trim() && !!last.reasoning) {
				this.displayMessages = this.displayMessages.slice(0, -1)
			}
		}
	}

	// Roll a turn that produced nothing usable back out of the transcript and
	// hand its text back to the composer for editing/resending. `restoreToInput`
	// is false when a queued message is about to take over (a user cancel with
	// something queued) — then the rolled-back prompt is dropped rather than
	// shoved back into the input, so the handoff to the queued message is clean.
	private restoreUnsentTurn = (
		displayLenAfterUser: number,
		modelLenAfterUser: number,
		instructions: string,
		pastes: PasteAttachment[],
		restoreToInput: boolean = true
	) => {
		this.displayMessages = this.displayMessages.slice(0, displayLenAfterUser - 1)
		this.messages = this.messages.slice(0, modelLenAfterUser - 1)
		if (restoreToInput) {
			this.aiChatInput?.restoreInstructions(instructions, pastes)
		}
	}

	private chatRequest = async ({
		messages,
		abortController,
		callbacks,
		addedMessages,
		systemMessage: systemMessageOverride,
		onWebSearchUnavailable
	}: {
		messages: ChatCompletionMessageParam[]
		abortController: AbortController
		callbacks: ToolCallbacks & {
			onNewToken: (token: string) => void
			onMessageEnd: () => void
		}
		// Caller-owned accumulator so partial output survives an abort/throw.
		addedMessages?: ChatCompletionMessageParam[]
		systemMessage?: ChatCompletionSystemMessageParam
		onWebSearchUnavailable?: () => void
	}) => {
		try {
			// Use JS getters so runChatLoop re-reads tools/helpers/systemMessage/modelProvider
			// on each iteration. This is critical for changeModeTool (Navigator → Script/Flow)
			// which reassigns this.tools, this.helpers, this.systemMessage mid-loop.
			const self = this
			const result = await runChatLoop({
				messages,
				addedMessages,
				get systemMessage() {
					const base = systemMessageOverride ?? self.systemMessage
					// Inject the attached-files roster at request time (re-read each iteration)
					// so it always reflects the live file list without reactive bookkeeping.
					if (self.mode === AIMode.GLOBAL && self.attachedFiles.count > 0) {
						return appendAttachedFilesRoster(base, self.attachedFiles)
					}
					return base
				},
				get tools() {
					return self.tools
				},
				get helpers() {
					return self.helpers
				},
				abortController,
				callbacks,
				get modelProvider() {
					return getCurrentModel()
				},
				get webSearch() {
					return isWebSearchEnabledForProvider(getCurrentModel().provider)
				},
				// Build the proxy clients against the operating workspace, not the global
				// singleton: a session deliberately leaves workspaceStore untouched, so the
				// singleton (init'd only on global workspace changes) would route the LLM
				// request through the navigation workspace's /ai/proxy instead of the
				// session's — sending it to the wrong workspace's AI credentials.
				get clients() {
					const ws = self.operatingWorkspace ?? ''
					return {
						openai: workspaceAIClients.createOpenaiClient(ws),
						anthropic: workspaceAIClients.createAnthropicClient(ws)
					}
				},
				workspace: this.operatingWorkspace ?? '',
				skipResponsesApi: this.skipResponsesApi,
				onSkipResponsesApi: () => {
					this.skipResponsesApi = true
				},
				onWebSearchUnavailable,
				getPendingUserMessage: () => {
					const pendingPrompt = this.pendingPrompt
					if (!pendingPrompt) return undefined
					this.pendingPrompt = ''
					if (this.mode === AIMode.SCRIPT) {
						return prepareScriptUserMessage(pendingPrompt, this.contextManager.getSelectedContext())
					} else if (this.mode === AIMode.FLOW) {
						return prepareFlowUserMessage(
							pendingPrompt,
							this.flowAiChatHelpers!.getFlowAndSelectedId(),
							[],
							this.flowAiChatHelpers!.inlineScriptSession
						)
					} else if (this.mode === AIMode.NAVIGATOR) {
						return prepareNavigatorUserMessage(pendingPrompt)
					} else if (this.mode === AIMode.GLOBAL) {
						return prepareGlobalUserMessage(
							pendingPrompt,
							this.contextManager.getSelectedContext(),
							{ workspace: this.operatingWorkspace }
						)
					}
					return undefined
				},
				onBeforeIteration: async (tools) => {
					for (const tool of tools) {
						if (tool.setSchema) {
							await tool.setSchema(this.helpers)
						}
					}
				}
			})
			return result
		} catch (err) {
			console.log('chatRequest error', err)
			console.error('chatRequest error', err)
			callbacks.onMessageEnd()
			this.cancelLoadingTools('Error')
			if (!abortController.signal.aborted) {
				throw err
			}
		}
	}

	sendInlineRequest = async (instructions: string, selectedCode: string, selection: Selection) => {
		// Validate inputs
		if (!instructions.trim()) {
			throw new Error('Instructions are required')
		}
		// Use a separate abort controller for inline requests to avoid interfering with main chat
		this.inlineAbortController = new AbortController()
		const lang = this.scriptEditorOptions?.lang ?? 'bun'
		const selectedContext: ContextElement[] = [...this.contextManager.getSelectedContext()]
		const startLine = selection.startLineNumber
		const endLine = selection.endLineNumber
		selectedContext.push({
			type: 'code_piece',
			lang,
			title: `L${startLine}-L${endLine}`,
			startLine,
			endLine,
			content: selectedCode
		})

		const systemMessage: ChatCompletionSystemMessageParam = {
			role: 'system',
			content: prepareInlineChatSystemPrompt(lang, {
				workflowAsCode: this.scriptEditorOptions?.workflowAsCode ?? false
			})
		}

		let reply = ''

		try {
			const userMessage = prepareScriptUserMessage(instructions, selectedContext)
			const messages = [userMessage]

			const params = {
				messages,
				abortController: this.inlineAbortController,
				callbacks: {
					onNewToken: (token: string) => {
						reply += token
					},
					onMessageEnd: () => {},
					setToolStatus: () => {},
					removeToolStatus: () => {}
				},
				systemMessage
			}

			await this.chatRequest({ ...params })

			// Validate we received a response
			if (!reply.trim()) {
				throw new Error('AI response was empty')
			}

			// Try to extract new code from response
			const newCodeMatch = reply.match(/<new_code>([\s\S]*?)<\/new_code>/i)
			if (newCodeMatch && newCodeMatch[1]) {
				const code = newCodeMatch[1].trim()
				if (!code) {
					throw new Error('AI response contained empty code block')
				}
				return code
			}

			// Fallback: try to take everything after the last <new_code> tag
			const lastNewCodeMatch = reply.match(/<new_code>([\s\S]*)/i)
			if (lastNewCodeMatch && lastNewCodeMatch[1]) {
				const code = lastNewCodeMatch[1].trim().replace(/```/g, '')
				if (!code) {
					throw new Error('AI response contained empty code block')
				}
				return code
			}

			// If no code tags found, throw error with helpful message
			throw new Error('AI response did not contain valid code. Please try rephrasing your request.')
		} catch (error) {
			// if abort controller is aborted, don't throw an error
			if (this.inlineAbortController?.signal.aborted) {
				return
			}
			console.error('Unexpected error in sendInlineRequest:', error)
			throw new Error('An unexpected error occurred. Please try again.')
		}
	}

	// Optional pre-flight hook called once per send, after the user's message
	// bubble + loading indicator are shown optimistically but before the request
	// goes out. Sessions use this to commit/materialise the workspace (creating a
	// staged fork via the API) so the first message targets the correct workspace.
	beforeSend?: () => Promise<void> | void
	afterFirstTurnSaved?: () => Promise<void> | void

	sendRequest = async (
		options: {
			removeDiff?: boolean
			addBackCode?: boolean
			instructions?: string
			pastes?: PasteAttachment[]
			mode?: AIMode
			lang?: ScriptLang | 'bunnative'
			isPreprocessor?: boolean
		} = {}
	) => {
		// Returns whether the input was consumed: true when it was sent as a chat
		// turn OR handled as a local built-in command, false when it was dropped
		// without being acted on (mode hidden, empty, beforeSend failed). The
		// queue flush restores the queued message only on false, so a consumed
		// command isn't re-queued and re-fired into the next conversation.
		const requestedMode = options.mode ?? this.mode
		if (!isAIModeVisible(requestedMode)) {
			return false
		}
		this.changeMode(requestedMode, undefined, {
			lang: options.lang,
			isPreprocessor: options.isPreprocessor
		})
		if (options.instructions) {
			this.instructions = options.instructions
		}
		if (!this.instructions.trim()) {
			return false
		}
		// Built-in session commands run locally instead of becoming a chat turn.
		// Intercepted here — before the beforeSend workspace commit, file regrants,
		// and skill expansion. Scoped to session chat GLOBAL mode, where the
		// slash-command UI lives. Return true (consumed, not dropped) so that a
		// command flushed from the queue isn't restored and re-fired into the next
		// conversation.
		if (this.isSessionChat && this.mode === AIMode.GLOBAL) {
			const trimmed = this.instructions.trim()
			// `/compact`: summarize the conversation locally to free up context.
			if (COMPACT_COMMAND_RE.test(trimmed)) {
				this.instructions = ''
				await this.compactManually()
				return true
			}
			// `/clear`: save the conversation to history and start a fresh chat.
			if (CLEAR_COMMAND_RE.test(trimmed)) {
				this.instructions = ''
				await this.saveAndClear()
				return true
			}
		}
		// Re-grant any locked File System Access handles within this send gesture, so the
		// file tools can read the live files. requestPermission() needs a user gesture, and
		// this runs before the first await/network call while the Send click is still active.
		// Attachment upkeep must never block the send — affected files just stay locked/stale
		// and the tools report their status to the model.
		try {
			await this.attachedFiles.regrantLocked()
			// Re-enumerate linked folders so on-disk changes (renamed/added/removed/edited
			// files) are reflected in the roster + indexes before this turn runs.
			await this.attachedFiles.refreshFolders()
		} catch (e) {
			console.error('Attached-files upkeep failed before send', e)
		}
		// beforeSend runs sequential API calls (session materialise + workspace fork
		// creation) that can take seconds. Show the user bubble and loading indicator
		// optimistically before it so the input doesn't just clear into a void.
		// Context elements and the snapshot are attached after beforeSend (see below).
		const isFirstUserTurn = !this.displayMessages.some((message) => message.role === 'user')
		const pastes = options.pastes ?? []
		const optimisticIndex = this.displayMessages.length
		this.loading = true
		// Create the abort controller before the (possibly slow) beforeSend pre-flight,
		// not after: the loading indicator below exposes Stop/Escape during "Creating
		// workspace fork...", and those call cancel() → abortController.abort(). Without a
		// controller here that abort would hit nothing and the request would still fire
		// once the pre-flight resolves; the pre-flight-cancel check after beforeSend honours it.
		this.abortController = new AbortController()
		this.displayMessages = [
			...this.displayMessages,
			{
				role: 'user',
				content: this.instructions,
				pastes: pastes.length > 0 ? pastes : undefined,
				index: this.messages.length // matching with actual messages index. not -1 because it's not yet added to the messages array
			}
		]
		// Undo the optimistic bubble + loading/label. Shared by the beforeSend-failure and
		// pre-flight-cancel paths below; the input keeps the message text either way.
		const rollbackOptimisticSend = () => {
			this.displayMessages = this.displayMessages.filter((_, i) => i !== optimisticIndex)
			this.loading = false
			this.loadingLabel = undefined
		}
		if (this.beforeSend) {
			try {
				await this.beforeSend()
			} catch (e) {
				// beforeSend commits the session's workspace before the first
				// message hits the backend. If it throws, sending anyway would
				// silently target the wrong workspace (typically the parent), so
				// abort and tell the user — their message text stays in the input.
				console.error('AIChatManager beforeSend hook failed', e)
				rollbackOptimisticSend()
				sendUserToast(
					`Could not prepare the session before sending: ${
						e instanceof Error ? e.message : String(e)
					}. Your message was not sent — please try again.`,
					true
				)
				return false
			}
		}
		// Session chats commit their workspace in beforeSend; skills must match the
		// committed workspace before the system prompt is sent.
		if (this.mode === AIMode.GLOBAL) {
			await this.refreshGlobalSkills(this.operatingWorkspace ?? '')
		}
		// Stop/Escape during the beforeSend pre-flight aborted this send before any
		// request went out. Mirror the main "cancelled before usable output" recovery:
		// roll the optimistic turn back, then either hand off to a queued message (the
		// input cleared the composer on send, so a deliberate cancel with a queued
		// message auto-sends it) or restore this prompt to the composer so it isn't lost.
		if (this.abortController.signal.aborted) {
			rollbackOptimisticSend()
			if (this.wasCancelledByUser() && this.queuedMessage) {
				const next = this.queuedMessage
				this.queuedMessage = ''
				const accepted = await this.sendRequest({ instructions: next })
				if (accepted === false) this.queuedMessage = next
			} else {
				this.aiChatInput?.restoreInstructions(this.instructions, pastes)
			}
			return true
		}
		// Declared outside `try` so the catch can recover what the loop produced
		// before a failure: the structured messages and the latest streamed text
		// that never became one.
		const collectedMessages: ChatCompletionMessageParam[] = []
		let partialReply = ''
		// Once an outcome branch (commit/restore) took over, a later throw (e.g.
		// from saveChat) must not make the catch commit the turn a second time.
		let turnOutcomeHandled = false
		let webSearchUnavailable = false
		// Gates the queued-message flush below: only a cleanly committed turn
		// auto-sends the next queued message. Cancel, error, and empty-response
		// rollbacks leave it false so queued text is restored to the input.
		let turnCommittedCleanly = false
		try {
			const oldSelectedContext = this.contextManager?.getSelectedContext() ?? []
			if (this.mode === AIMode.SCRIPT || this.mode === AIMode.FLOW) {
				this.contextManager?.updateContextOnRequest(options)
			}
			// loading + abortController were set optimistically before beforeSend, above.
			this.#automaticScroll = true

			const model = tryGetCurrentModel()
			if (model) {
				WorkspaceService.logAiChat({
					workspace: this.operatingWorkspace ?? '',
					requestBody: {
						session_id: this.historyManager.getCurrentChatId(),
						provider: model.provider,
						model: model.model,
						mode: this.mode
					}
				}).catch(() => {})
			}

			if (this.mode === AIMode.FLOW && !this.flowAiChatHelpers) {
				throw new Error('No flow helpers found')
			}

			let snapshot:
				| { type: 'flow'; value: ExtendedOpenFlow }
				| { type: 'app'; value: number }
				| undefined = undefined
			if (this.mode === AIMode.FLOW) {
				snapshot = { type: 'flow', value: this.flowAiChatHelpers!.getFlowAndSelectedId().flow }
				this.flowAiChatHelpers!.setSnapshot(snapshot.value)
			} else if (this.mode === AIMode.APP) {
				snapshot = { type: 'app', value: this.appAiChatHelpers!.snapshot() }
			}

			// Attach the enrichments that are only known after beforeSend (selected
			// context + snapshot) to the optimistic user message pushed before it.
			this.displayMessages = this.displayMessages.map((m, i) =>
				i === optimisticIndex
					? {
							...m,
							contextElements:
								this.mode === AIMode.SCRIPT ||
								this.mode === AIMode.FLOW ||
								this.mode === AIMode.GLOBAL
									? oldSelectedContext
									: undefined,
							snapshot
						}
					: m
			)
			// For restoreUnsentTurn: the compact composer form (with paste tokens),
			// not the expanded LLM text, plus the rollback anchor after the user turn.
			const sentInstructions = this.instructions
			const sentPastes = pastes
			// The LLM gets the full pasted content; the display message above keeps
			// the compact tokens + registry so the bubble can render/expand chips.
			const oldInstructions = expanded(chatDraft(this.instructions, pastes))
			// Deliver background-job completions to the model as a preamble on this
			// turn (notify-only wake). Folded into the model-facing text only — the
			// display bubble keeps this.instructions, and no extra message is added, so
			// the display↔messages index pairing above stays intact. Ephemeral.
			const jobNotesPreamble =
				this.mode === AIMode.GLOBAL && this.pendingJobNotes.length > 0
					? this.pendingJobNotes.join('\n\n') + '\n\n'
					: ''
			if (jobNotesPreamble) this.pendingJobNotes = []
			const modelInstructions =
				this.mode === AIMode.GLOBAL
					? jobNotesPreamble + this.expandGlobalSkillCommand(oldInstructions)
					: oldInstructions
			this.instructions = ''

			if (this.mode === AIMode.SCRIPT && !this.scriptEditorOptions && !options.lang) {
				throw new Error('No script options passed')
			}

			let userMessage: ChatCompletionMessageParam = {
				role: 'user',
				content: ''
			}
			switch (this.mode) {
				case AIMode.FLOW:
					userMessage = prepareFlowUserMessage(
						oldInstructions,
						this.flowAiChatHelpers!.getFlowAndSelectedId(),
						oldSelectedContext,
						this.flowAiChatHelpers!.inlineScriptSession
					)
					break
				case AIMode.NAVIGATOR:
					userMessage = prepareNavigatorUserMessage(oldInstructions)
					break
				case AIMode.ASK:
					userMessage = prepareAskUserMessage(oldInstructions)
					break
				case AIMode.SCRIPT:
					userMessage = prepareScriptUserMessage(oldInstructions, oldSelectedContext)
					break
				case AIMode.API:
					userMessage = prepareApiUserMessage(oldInstructions)
					break
				case AIMode.GLOBAL:
					userMessage = prepareGlobalUserMessage(modelInstructions, oldSelectedContext, {
						workspace: this.operatingWorkspace
					})
					break
				case AIMode.APP:
					userMessage = prepareAppUserMessage(
						oldInstructions,
						this.appAiChatHelpers?.getSelectedContext(),
						oldSelectedContext
					)
					break
			}

			// Size of the request about to go out: contextTokens (provider report
			// when current, fresh chars/4 estimate otherwise) plus the message
			// being added below. Must be read BEFORE the push — the estimate path
			// covers the stored history, so pushing first would double-count the
			// new message.
			const projectedContextTokens = this.contextTokens + this.estimateMessagesTokens([userMessage])

			this.messages.push(userMessage)
			await this.historyManager.saveChat(
				this.displayMessages,
				this.messages,
				this.contextUsage,
				this.modifiedItems ? [...this.modifiedItems] : undefined
			)

			this.replyReveal.reset()
			this.reasoningReveal.reset()
			this.currentReply = ''
			this.currentReasoning = ''
			this.currentReasoningActive = false

			// Compaction trigger. Without a known context window there is no limit
			// to enforce, so compaction stays off rather than guessing one.
			const contextWindow = model ? getKnownModelContextWindow(model.model) : undefined
			if (
				contextWindow !== undefined &&
				projectedContextTokens >= contextWindow * COMPACTION_TRIGGER_RATIO
			) {
				// Preferred path: summarize the older prefix, keep the recent tail.
				const summarized = await this.summarizeAndCompact(contextWindow)
				// A Stop during the in-flight summary aborts this turn's controller;
				// summarizeAndCompact then returns false without touching history. Skip
				// the drop-oldest fallback (and its save) — it would destructively
				// compact a conversation the user only meant to cancel, and the request
				// can't run on an aborted controller anyway. The cancel path below rolls
				// the pushed turn back cleanly on its own.
				if (!this.abortController?.signal.aborted) {
					if (!summarized) {
						// Fallback when summarization isn't worthwhile or fails: drop the
						// oldest messages. A report stays meaningful only debited by what
						// was dropped; the estimate path needs no bookkeeping — the next
						// read re-estimates the compacted history. chars/4 can
						// underestimate the freed tokens, which errs toward compacting
						// again — never toward overflowing.
						const freed = this.compactOldestMessages(
							projectedContextTokens - contextWindow * COMPACTION_TARGET_RATIO
						)
						if (this.contextUsage !== undefined) {
							this.contextUsage = Math.max(0, this.contextUsage - freed)
						}
					}
					await this.historyManager.saveChat(
						this.displayMessages,
						this.messages,
						this.contextUsage,
						this.modifiedItems ? [...this.modifiedItems] : undefined
					)
				}
			}
			// Rollback anchors for restoreUnsentTurn: captured after compaction so
			// they index into the (possibly compacted) stored history. The summary
			// path shrinks displayMessages too, so the display anchor must also be
			// read here, not before compaction.
			const modelLenAfterUser = this.messages.length
			const displayLenAfterUser = this.displayMessages.length

			const params: {
				messages: ChatCompletionMessageParam[]
				abortController: AbortController
				callbacks: ToolCallbacks & {
					onNewToken: (token: string) => void
					onMessageEnd: () => void
				}
			} = {
				messages: [...this.messages],
				abortController: this.abortController,
				callbacks: {
					onNewToken: (token) => this.replyReveal.push(token),
					onReasoningDelta: (token) => this.reasoningReveal.push(token),
					onReasoningStart: () => (this.currentReasoningActive = true),
					onMessageEnd: () => {
						// Drain any un-revealed backlog into currentReply first, so the reads
						// below see the full text. This funnel covers clean completion, tool
						// boundaries, and abort/error — flush-before-read is the invariant that
						// keeps text from being lost or duplicated on any exit path.
						this.replyReveal.flush()
						this.reasoningReveal.flush()
						// Keep the streamed text for the abort/error paths. Non-empty only:
						// parsers flush (and reset) when a tool call starts after text, and
						// the catch's later empty call would wipe it — stale keeps are
						// deduped in commitInterruptedTurn.
						if (this.currentReply) {
							partialReply = this.currentReply
						}
						if (this.currentReply || this.currentReasoning) {
							this.displayMessages = [
								...this.displayMessages,
								{
									role: 'assistant',
									content: this.currentReply,
									...(this.currentReasoning ? { reasoning: this.currentReasoning } : {}),
									contextElements:
										this.mode === AIMode.SCRIPT
											? oldSelectedContext.filter((c) => c.type === 'code')
											: undefined
								}
							]
						}
						this.currentReply = ''
						this.currentReasoning = ''
						this.currentReasoningActive = false
					},
					setToolStatus: this.applyToolStatus,
					// Job-tracking hooks enable detach-into-background; wire them only in
					// GLOBAL mode (global chat + sessions). In-editor script/flow/pipeline
					// chats leave these undefined, so their test runs keep blocking.
					...(this.mode === AIMode.GLOBAL
						? {
								onJobStarted: (job) => this.registerJob(job),
								onJobStatus: (jobId, update) => this.updateJob(jobId, update),
								onJobDetached: (jobId) => this.markJobDetached(jobId)
							}
						: {}),
					removeToolStatus: (id) => {
						const existingIdx = this.displayMessages.findIndex(
							(m) => m.role === 'tool' && m.tool_call_id === id
						)
						if (existingIdx !== -1) {
							this.displayMessages.splice(existingIdx, 1)
							this.displayMessages = [...this.displayMessages]
						}
					},
					requestConfirmation: this.requestConfirmation,
					shouldAutoAcceptToolConfirmations: () => this.autoAcceptToolConfirmationsActive,
					requestUserQuestion: this.requestUserQuestion,
					onItemModified: (kind, path) => this.recordModifiedItem(kind, path),
					onItemDeployed: (kind, from, to) => void this.renameModifiedItem(kind, from, to),
					onItemDiscarded: (kind, path) => void this.removeModifiedItem(kind, path)
				}
			}

			if (this.mode === AIMode.API && this.apiTools.length === 0) {
				await this.loadApiTools()
			}

			const result = await this.chatRequest({
				...params,
				addedMessages: collectedMessages,
				onWebSearchUnavailable: () => {
					webSearchUnavailable = true
				}
			})
			const wasAborted = this.abortController?.signal.aborted ?? false
			// Pure reasoning doesn't count as usable: it's not replayed as context,
			// so a reasoning-only turn is as unsent as a literally empty one.
			const hasUsableOutput =
				truncateToToolPairedPrefix(collectedMessages).length > 0 || !!partialReply.trim()
			turnOutcomeHandled = true

			if (wasAborted && hasUsableOutput) {
				// Interrupted after some output: keep it so a follow-up like
				// "continue" picks up from there.
				this.commitInterruptedTurn(collectedMessages, partialReply)
				if (this.autoAcceptEditsActive) {
					this.acceptPendingFlowEdits()
				}
				// The report from the last completed iteration still describes the
				// stored history it was sent with (the kept partial tail is a small
				// undercount the trigger headroom absorbs). Without one, clear the
				// stale value — readers estimate via contextTokens.
				this.contextUsage = result?.lastIterationUsage
					? result.lastIterationUsage.prompt + result.lastIterationUsage.completion
					: undefined
				await this.historyManager.saveChat(
					this.displayMessages,
					this.messages,
					this.contextUsage,
					this.modifiedItems ? [...this.modifiedItems] : undefined
				)
				// Still counts as the saved first turn — skipping the hook here would
				// permanently miss it (the next turn isn't "first" anymore).
				if (isFirstUserTurn && this.afterFirstTurnSaved) {
					void Promise.resolve(this.afterFirstTurnSaved()).catch((e) => {
						console.error('AIChatManager afterFirstTurnSaved hook failed', e)
					})
				}
			} else if (wasAborted || !hasUsableOutput) {
				// Cancelled before anything usable, or the model returned nothing
				// (or only reasoning) — treat the turn as unsent (matches Claude Code).
				// contextUsage is left as-is: the turn is rolled back, so the last
				// report (pre-turn, possibly debited by compaction) still stands.
				// When the user cancelled with a message queued, that message is
				// about to auto-send (see the flush below) — drop the rolled-back
				// prompt instead of restoring it to the input so the handoff is clean.
				const willAutoSendQueued = this.wasCancelledByUser() && !!this.queuedMessage
				this.restoreUnsentTurn(
					displayLenAfterUser,
					modelLenAfterUser,
					sentInstructions,
					sentPastes,
					!willAutoSendQueued
				)
				if (this.displayMessages.length === 0) {
					// saveChat no-ops on an empty transcript; the chat persisted earlier
					// this turn would linger in history and resurface the rolled-back
					// user message on reload. Remove it instead.
					this.historyManager.deletePastChat(this.historyManager.getCurrentChatId())
				} else {
					await this.historyManager.saveChat(
						this.displayMessages,
						this.messages,
						this.contextUsage,
						this.modifiedItems ? [...this.modifiedItems] : undefined
					)
				}
				if (!wasAborted) {
					sendUserToast('The model returned no response — your message was restored to the input.')
				}
			} else {
				// Clean turn with output → commit as-is.
				this.messages = [...this.messages, ...collectedMessages]
				// The provider's report describes the stored history exactly:
				// compaction mutates it before sending, so what was sent IS what is
				// stored — no anchoring or index bookkeeping needed. Without a
				// report, clear the now-stale value — readers estimate via
				// contextTokens.
				this.contextUsage = result?.lastIterationUsage
					? result.lastIterationUsage.prompt + result.lastIterationUsage.completion
					: undefined
				if (this.autoAcceptEditsActive) {
					this.acceptPendingFlowEdits()
				}
				await this.historyManager.saveChat(
					this.displayMessages,
					this.messages,
					this.contextUsage,
					this.modifiedItems ? [...this.modifiedItems] : undefined
				)
				// Only this branch is a clean send: the queued-message flush below
				// auto-sends the next message after it (set after saveChat so a
				// persistence failure falls through to the restore path instead).
				turnCommittedCleanly = true
				if (isFirstUserTurn && this.afterFirstTurnSaved) {
					void Promise.resolve(this.afterFirstTurnSaved()).catch((e) => {
						console.error('AIChatManager afterFirstTurnSaved hook failed', e)
					})
				}
			}
		} catch (err) {
			console.error(err)
			// Request failure: keep the usable output as context for a follow-up.
			// Skipped when the throw came from post-outcome code (e.g. saveChat) —
			// re-committing would duplicate the turn's messages.
			if (!turnOutcomeHandled) {
				this.commitInterruptedTurn(collectedMessages, partialReply)
				// Any prior report no longer describes the history (a partial turn
				// was just committed); clear it so readers estimate instead. When
				// the failure WAS a context-length error, that high estimate forces
				// compaction on the next send instead of failing the same way again.
				this.contextUsage = undefined
				try {
					await this.historyManager.saveChat(
						this.displayMessages,
						this.messages,
						this.contextUsage,
						this.modifiedItems ? [...this.modifiedItems] : undefined
					)
				} catch (saveErr) {
					console.error('Failed to persist partial chat after error', saveErr)
				}
				this.flagLastMessageAsError()
			}
			sendUserToast(getSendRequestErrorMessage(err, webSearchUnavailable), true)
		} finally {
			this.loading = false
			// Turn teardown: cancel any in-flight reveal frame and drop leftover
			// backlog. onMessageEnd already flushed on every outcome, so this only
			// releases the loop; it never discards uncommitted text.
			this.replyReveal.reset()
			this.reasoningReveal.reset()
			// Refresh the free-tier usage meter after every turn (success or error), and
			// let the turn that exhausts the grant flip to the exhausted state live.
			void refreshFreeTierUsage(this.operatingWorkspace)
		}
		// Flush the queued message. Send it after a cleanly committed turn OR a
		// deliberate user cancel (Esc / Stop) — in both cases the user is ready
		// to move on, so it sends automatically. A genuine error, an
		// empty-response rollback, or a programmatic cancel (panel teardown,
		// save-and-clear) leaves it in place as a card so it isn't fired into a
		// failed or torn-down turn.
		if ((turnCommittedCleanly || this.wasCancelledByUser()) && this.queuedMessage) {
			const next = this.queuedMessage
			this.queuedMessage = ''
			const accepted = await this.sendRequest({ instructions: next })
			if (accepted === false) {
				// The auto-send bailed before becoming a turn (e.g. beforeSend
				// failed); keep it as the queued message instead of losing it.
				this.queuedMessage = next
			}
		}
		// A background job may have finished mid-turn: its note missed this turn's
		// preamble (captured at the start) and the poller skipped auto-resume while
		// we were loading. Now that we're idle, deliver it via an auto-resume. Skips
		// itself if the queued-message flush above already carried the notes.
		void this.#maybeAutoResumeFromJobs()
		return true
	}

	// True when the current turn's controller was aborted by a deliberate user
	// cancel (Esc / Stop), as opposed to a programmatic cancel (panel teardown,
	// save-and-clear) or no abort at all. Gates the queued-message auto-send.
	private wasCancelledByUser(): boolean {
		const signal = this.abortController?.signal
		return !!signal?.aborted && signal.reason === USER_CANCEL_REASON
	}

	cancel = (reason?: string) => {
		for (const confirmationCallback of this.confirmationCallbacks.values()) {
			confirmationCallback(false)
		}
		this.confirmationCallbacks.clear()
		for (const resolveQuestion of this.userQuestionCallbacks.values()) {
			resolveQuestion(undefined)
		}
		this.userQuestionCallbacks.clear()
		const cancelReason = reason ?? USER_CANCEL_REASON
		console.log('cancelling request:', {
			reason: cancelReason,
			abortController: this.abortController
		})
		this.abortController?.abort(cancelReason)
		this.cancelLoadingTools()
	}

	cancelInlineRequest = (reason?: string) => {
		const cancelReason = reason ?? 'inline_cancelled'
		console.log('cancelling inline request:', {
			reason: cancelReason,
			inlineAbortController: this.inlineAbortController
		})
		this.inlineAbortController?.abort(cancelReason)
	}

	restartGeneration = (
		displayMessageIndex: number,
		newContent?: string,
		pastes?: PasteAttachment[]
	) => {
		const userMessage = this.displayMessages[displayMessageIndex]

		if (!userMessage || userMessage.role !== 'user') {
			throw new Error('No user message found at the specified index')
		}

		// Remove all messages including and after the specified user message
		this.displayMessages = this.displayMessages.slice(0, displayMessageIndex)

		// Find corresponding message in actual messages and remove it and everything after it
		let actualMessageIndex = this.messages.findIndex((_, i) => i === userMessage.index)

		if (actualMessageIndex === -1) {
			throw new Error('No actual user message found to restart from')
		}

		this.messages = this.messages.slice(0, actualMessageIndex)

		// The last report described the pre-rewind history; clear it. Readers
		// fall back to estimating the rewound history (contextTokens), so the
		// compaction trigger stays armed — e.g. for Retry after a context-length
		// error, which rewinds through here.
		this.contextUsage = undefined

		// Resend the request with the same instructions
		this.instructions = newContent ?? userMessage.content
		this.sendRequest({ pastes: pastes ?? userMessage.pastes })
	}

	fix = () => {
		if (!this.open) {
			this.toggleOpen()
		}
		this.changeMode(AIMode.SCRIPT)
		this.instructions = 'Fix the error'
		this.contextManager?.setFixContext()
		this.sendRequest()
	}

	addSelectedLinesToContext = (
		lines: string,
		startLine: number,
		endLine: number,
		moduleId?: string
	) => {
		if (!this.open) {
			this.toggleOpen()
		}
		if (!moduleId) {
			this.changeMode(AIMode.SCRIPT)
		}
		this.contextManager?.addSelectedLinesToContext(lines, startLine, endLine, moduleId)
		this.focusInput()
	}

	saveAndClear = async () => {
		this.cancel('saveAndClear')
		// Drop any message queued in this conversation so it can't auto-send into
		// the fresh chat or linger as a card across the switch.
		this.queuedMessage = ''
		// The tray + poller belong to the conversation being left; the just-saved
		// chat keeps its persisted jobs (save() omits the arg → fallback preserves).
		this.clearBackgroundJobs()
		await this.historyManager.save(
			this.displayMessages,
			this.messages,
			this.contextUsage,
			this.modifiedItems ? [...this.modifiedItems] : undefined
		)
		this.displayMessages = []
		this.messages = []
		this.contextUsage = undefined
		// The mask belongs to the conversation just saved — the fresh chat starts
		// its own (empty) tracking; carrying entries over would claim the previous
		// conversation's edits for the new one. Untracked chats stay untracked.
		if (this.modifiedItems) this.modifiedItems = new SvelteSet()
		// In an AI session, linked files are session-scoped: they persist across conversations
		// (cleared only when the session is deleted). The ephemeral global side-panel chat has no
		// session, so "New chat" must clear them — otherwise the next, unrelated conversation
		// would still get the previous file roster and could read/search it.
		if (!this.isSessionChat) this.attachedFiles.clear()
		this.onChatRotated?.(this.historyManager.getCurrentChatId())
	}

	loadPastChat = async (id: string) => {
		const chat = this.historyManager.loadPastChat(id)
		if (chat) {
			// Drop any message queued in the current conversation so it doesn't
			// auto-send into the loaded one or linger as a card across the switch.
			this.queuedMessage = ''
			// Stop the poller for the conversation being left before swapping in the
			// loaded chat's jobs below.
			this.clearBackgroundJobs()
			// Same isolation as saveAndClear: the ephemeral global chat's attachments belong to
			// the conversation being left, not the one being loaded; sessions keep them.
			if (!this.isSessionChat) this.attachedFiles.clear()
			this.displayMessages = chat.displayMessages
			this.messages = chat.actualMessages
			this.contextUsage = normalizeContextUsage(chat.contextUsage)
			// Seed the modified-items mask from the stored chat. A session's Edits
			// surface is scoped strictly to what this session edited, so it must never
			// fall back to showing every draft in the (possibly forked) workspace: a
			// legacy chat with no stored mask seeds an empty tracked set, not undefined.
			// The global side-panel chat never tracks, so leave it untouched there.
			if (this.isSessionChat) {
				const stored = this.historyManager.getModifiedItems(id)
				this.modifiedItems = new SvelteSet(stored ?? [])
			}
			// Rebuild the jobs tray from the loaded chat, and re-attach the poller to
			// any job that was still in flight when it was last persisted.
			const storedJobs = this.historyManager.getBackgroundJobs(id)
			this.backgroundJobs = storedJobs ? storedJobs.map((j) => ({ ...j })) : []
			for (const j of this.backgroundJobs) {
				if (this.isJobNonTerminal(j.status)) j.detached = true
			}
			if (this.backgroundJobs.length > 0) this.backgroundJobs = [...this.backgroundJobs]
			this.#ensureJobPoller()
			this.#automaticScroll = true
			this.onChatRotated?.(id)
		}
	}

	get automaticScroll() {
		return this.#automaticScroll
	}

	disableAutomaticScroll = () => {
		this.#automaticScroll = false
	}

	enableAutomaticScroll = () => {
		this.#automaticScroll = true
	}

	generateStep = async (moduleId: string, lang: ScriptLang, instructions: string) => {
		if (!this.flowAiChatHelpers) {
			throw new Error('No flow helpers found')
		}
		this.flowAiChatHelpers.selectStep(moduleId)
		await this.sendRequest({
			instructions: instructions,
			mode: AIMode.SCRIPT,
			lang: lang,
			isPreprocessor: moduleId === 'preprocessor'
		})
	}

	listenForContextChange = (dbSchemas: DBSchemas, workspaceStore: string | undefined) => {
		if (this.mode === AIMode.SCRIPT && this.scriptEditorOptions) {
			this.contextManager.updateAvailableContext(
				this.scriptEditorOptions,
				dbSchemas,
				workspaceStore ?? '',
				true, // toolSupport: reasoning no longer disables DB/tool context
				untrack(() => this.contextManager.getSelectedContext())
			)
		} else if (this.mode === AIMode.FLOW && this.flowOptions) {
			this.contextManager.updateAvailableContextForFlow(
				this.flowOptions,
				dbSchemas,
				workspaceStore ?? '',
				true, // toolSupport: reasoning no longer disables DB/tool context
				untrack(() => this.contextManager.getSelectedContext())
			)
		} else if (this.mode === AIMode.GLOBAL) {
			this.contextManager.updateAvailableContextForGlobal(
				workspaceStore ?? '',
				untrack(() => this.contextManager.getSelectedContext())
			)
		}

		if (this.scriptEditorOptions) {
			this.contextManager.setScriptOptions(this.scriptEditorOptions)
		}
	}

	listenForDbSchemasChanges = (dbSchemas: DBSchemas) => {
		this.displayMessages = ContextManager.updateDisplayMessages(
			untrack(() => this.displayMessages),
			dbSchemas
		)
	}

	listenForCurrentEditorChanges = (currentEditor: CurrentEditor) => {
		if (currentEditor && currentEditor.type === 'script') {
			this.scriptEditorApplyCode = async (code, opts) => {
				if (currentEditor && currentEditor.type === 'script') {
					currentEditor.hideDiffMode()
					await currentEditor.editor.reviewAndApplyCode(code, opts)
				}
			}
			this.scriptEditorShowDiffMode = () => {
				if (currentEditor && currentEditor.type === 'script') {
					currentEditor.showDiffMode()
				}
			}
			this.scriptEditorGetLintErrors = () => {
				if (currentEditor && currentEditor.type === 'script') {
					return currentEditor.editor.getLintErrors()
				}
				return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
			}
		} else {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
			this.scriptEditorGetLintErrors = undefined
		}

		return () => {
			this.scriptEditorApplyCode = undefined
			this.scriptEditorShowDiffMode = undefined
			this.scriptEditorGetLintErrors = undefined
		}
	}

	listenForSelectedIdChanges = (
		selectedId: string | undefined,
		flowStore: ExtendedOpenFlow,
		flowStateStore: FlowState,
		currentEditor: CurrentEditor
	) => {
		function getModule(id: string) {
			if (id === 'preprocessor') {
				return flowStore.value.preprocessor_module
			} else if (id === 'failure') {
				return flowStore.value.failure_module
			} else {
				return dfs(id, flowStore, false)[0]
			}
		}

		function getScriptOptions(id: string): ScriptOptions | undefined {
			const module = getModule(id)

			if (module && module.value.type === 'rawscript') {
				const moduleState: FlowModuleState | undefined = flowStateStore[module.id]

				const editorRelated =
					currentEditor && currentEditor.type === 'script' && currentEditor.stepId === module.id
						? {
								diffMode: currentEditor.diffMode,
								lastDeployedCode: currentEditor.lastDeployedCode,
								lastSavedCode: undefined
							}
						: {
								diffMode: false,
								lastDeployedCode: undefined,
								lastSavedCode: undefined
							}
				return {
					args: moduleState?.previewArgs ?? {},
					error:
						moduleState && !moduleState.previewSuccess
							? getStringError(moduleState.previewResult)
							: undefined,
					getCode: () => (module.value.type === 'rawscript' ? module.value.content : ''),
					lang: module.value.language,
					path: module.id,
					...editorRelated
				}
			}

			return undefined
		}

		if (selectedId) {
			const options = getScriptOptions(selectedId)
			if (options) {
				this.scriptEditorOptions = options
			}
		} else {
			this.scriptEditorOptions = undefined
		}

		untrack(() =>
			this.contextManager?.setSelectedModuleContext(
				selectedId,
				untrack(() => this.contextManager.getAvailableContext())
			)
		)

		return () => {
			this.scriptEditorOptions = undefined
		}
	}

	setFlowHelpers = (flowHelpers: FlowAIChatHelpers) => {
		this.flowAiChatHelpers = flowHelpers
		untrack(() => {
			if (this.autoAcceptEditsActive) {
				this.acceptPendingFlowEdits(flowHelpers)
			}
		})

		return () => {
			this.flowAiChatHelpers = undefined
		}
	}

	// Registered by the /pipeline editor while it is mounted. Rebuilds the global
	// tool set so the pipeline tools appear (and disappear on unregister). Pipeline
	// AI edits apply directly as drafts, so there is nothing to auto-accept.
	// Returns a cleanup that tears the registration back down.
	setPipelineHelpers = (pipelineHelpers: PipelineAIChatHelpers) => {
		this.pipelineAiChatHelpers = pipelineHelpers
		untrack(() => {
			if (this.mode === AIMode.GLOBAL) {
				this.configureGlobalMode()
			}
		})

		return () => {
			this.pipelineAiChatHelpers = undefined
			untrack(() => {
				if (this.mode === AIMode.GLOBAL) {
					this.configureGlobalMode()
				}
			})
		}
	}

	/**
	 * Refresh cached datatables from the app helpers (async)
	 * Creates one context element per table (not per datatable)
	 */
	refreshDatatables = async (): Promise<void> => {
		if (!this.appAiChatHelpers) {
			this.cachedDatatables = []
			return
		}

		try {
			const datatables = await this.appAiChatHelpers.listDatatableTables()
			this.cachedDatatables = flattenDatatablesToAppContextElements(datatables)
		} catch (err) {
			console.error('Failed to refresh datatables:', err)
			this.cachedDatatables = []
		}
	}

	/**
	 * Get available context elements for app mode (frontend files + backend runnables + datatables)
	 */
	getAppAvailableContext = (): ContextElement[] => {
		if (!this.appAiChatHelpers) {
			return []
		}

		const context: ContextElement[] = []

		// Add frontend files
		const frontendFiles = this.appAiChatHelpers.listFrontendFiles()
		for (const path of frontendFiles) {
			const content = this.appAiChatHelpers.getFrontendFile(path)
			if (content !== undefined) {
				context.push(createAppFrontendFileContextElement(path, content))
			}
		}

		// Add backend runnables
		const runnables = this.appAiChatHelpers.listBackendRunnables()
		for (const { key } of runnables) {
			const runnable = this.appAiChatHelpers.getBackendRunnable(key)
			if (runnable) {
				context.push(createAppBackendRunnableContextElement(key, runnable))
			}
		}

		// Add cached datatables
		context.push(...this.cachedDatatables)

		return context
	}

	setAppHelpers = (appHelpers: AppAIChatHelpers) => {
		this.appAiChatHelpers = appHelpers
		// Refresh datatables when app helpers are set (deferred to avoid loop)
		// Use setTimeout to ensure this runs after the effect completes
		if (this.appDatatablesRefreshTimeout) {
			clearTimeout(this.appDatatablesRefreshTimeout)
		}
		this.appDatatablesRefreshTimeout = setTimeout(() => {
			this.appDatatablesRefreshTimeout = undefined
			if (this.appAiChatHelpers === appHelpers) {
				void this.refreshDatatables()
			}
		}, 50)

		return () => {
			if (this.appDatatablesRefreshTimeout) {
				clearTimeout(this.appDatatablesRefreshTimeout)
				this.appDatatablesRefreshTimeout = undefined
			}
			if (this.appAiChatHelpers === appHelpers) {
				this.appAiChatHelpers = undefined
				this.cachedDatatables = []
			}
		}
	}

	cancelLoadingTools = (messageText: 'Canceled' | 'Error' = 'Canceled') => {
		this.displayMessages = this.displayMessages.map((message) => {
			if (message.role === 'tool' && message.isLoading) {
				return {
					...message,
					isLoading: false,
					content: messageText,
					error: messageText,
					userQuestion: message.userQuestion
						? { ...message.userQuestion, canceled: true }
						: undefined
				}
			}
			return message
		})
	}
}

export const aiChatManager = new AIChatManager()

// The singleton is constructed at import — before the logged-in email resolves
// — so it starts at the safe autonomy default and an unopened chat-history DB.
// Hydrate both from user-scoped storage once the email is known, and on any
// later user change. Registered only here (not in the constructor) so
// per-session managers don't accumulate never-removed callbacks.
//
// init() is email-gated and idempotent, so re-opening the scoped DB here
// (alongside AiChatLayout's mount-time init()) is harmless and lets the
// singleton self-heal on email change like the other user-scoped surfaces.
onUserChange(() => {
	aiChatManager.hydrateUserScopedAutonomy()
	void aiChatManager.historyManager.init()
})
