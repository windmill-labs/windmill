import { SvelteMap } from 'svelte/reactivity'
import { get } from 'svelte/store'
import { base } from '$lib/base'
import { AIChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { PipelineEditorState } from '$lib/components/assets/AssetGraph/pipelineEditorState.svelte'
import { initFlow } from '$lib/components/flows/flowStore.svelte'
import {
	AppService,
	FlowService,
	ScriptService,
	type Flow,
	type NewScript,
	type Script,
	type UserDraftOverlay
} from '$lib/gen'

// `get_draft=true` does NOT merge: the top-level fields stay the deployed
// payload and the user's draft rides in a sibling `.draft` pocket (with
// `is_draft` / `draft_saved_at` alongside). Hence the `saved.draft ?? saved`
// fall-throughs below prefer the draft, else the deployed payload.
// The generated `UserDraftOverlay` types `draft` permissively; locally it's a
// `NewScript` / `Flow`, so `Omit` and re-add the precise type (assignments from
// the response type still need an explicit cast).
type SavedScript = Omit<Script & UserDraftOverlay, 'draft'> & { draft?: NewScript }
type SavedFlow = Omit<Flow & UserDraftOverlay, 'draft'> & { draft?: Flow }
import type { HiddenRunnable } from '$lib/components/apps/types'
import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { userWorkspaces, workspaceStore } from '$lib/stores'
import { loadCopilot, copilotWorkspace } from '$lib/aiStore'
import { emptySchema, type StateStore } from '$lib/utils'
import {
	commitSessionWorkspace,
	deleteSession as deleteSessionState,
	ensureChatIdsSeeded,
	getEffectiveWorkspaceId,
	materializeTransient,
	sessionState,
	setGeneratedSessionSummary,
	setSessionChatId,
	setSessionPreviewCollapsed,
	setSessionPreviewSize,
	setSessionTabs,
	type Session
} from './sessionState.svelte'
import {
	SessionPreviewTabs,
	describePreview,
	hydratePreviewTabs,
	previewTargetForSessionTarget,
	selectPreviewTabsToClose
} from './sessionPreviewTabs.svelte'
import { matchPreviewPage, parsePreviewItemRoute, previewLocationLabel } from './previewRouter'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { armRestartOnFirstInteraction } from '$lib/userDraftToast'
import { applyDraftToRuntimeRawApp, runtimeRawAppToDraft, type RawAppDraft } from './appDraftCodec'
import {
	setDeployedInSessionHandler,
	setClosePreviewTabsHandler,
	setGetPreviewStatusHandler,
	setGetRuntimeLogsHandler,
	setGetDomHandler,
	setListAppRunsHandler,
	setScreenshotHandler,
	setOpenPagePreviewHandler,
	setOpenPreviewHandler
} from '$lib/components/copilot/chat/global/core'
import {
	formatRuntimeLogsForChat,
	formatAppRunsForChat,
	type RawAppRuntimeLogEntry,
	type RawAppRuntimeLogRequester,
	type RawAppRunSummary,
	type RawAppRunsProvider,
	type RawAppScreenshotRequester
} from '$lib/components/raw_apps/utils'
import type {
	RawAppDomQuery,
	RawAppDomRequester,
	RawAppDomResult
} from '$lib/components/raw_apps/rawAppDom'
import { getNonStreamingMetadataCompletion } from '$lib/components/copilot/lib'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

// Per-kind load state for a session's editor target. Pure state container the
// load methods write into; the editor-target gate reads it to decide between
// the loading overlay, the not-found state, and a remount of the heavy editor.
// `loadedPath` flips to the requested path only once the load settles (data
// ready), which is what lets the gate remount on data-ready rather than on the
// (synchronous) target swap.
export interface LoadSlot {
	loadedPath: string | undefined
	// The workspace `loadedPath`'s content was fetched from. A session can retarget
	// the same item path to a different fork, so the cache must key on both — else
	// the editor keeps the old workspace's content while save/deploy write to the new.
	loadedWorkspace: string | undefined
	loading: boolean
	notFound: boolean
}

export type SessionTargetKind = 'flow' | 'script' | 'raw_app'

// The live runtime value a raw-app editor cell binds. Legacy drag-and-drop apps
// are intentionally NOT hosted in the session preview (only code-based raw apps).
export interface RawAppRuntimeValue {
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	policy: any
	summary: string
	path: string
	custom_path?: string
	draft_path?: string
}
// The deployed baseline a raw-app cell diffs against (topbar Diff drawer).
export interface RawAppSavedValue {
	value: {
		files: Record<string, { code: string }>
		runnables: Record<string, HiddenRunnable>
	}
	draft?: any
	path: string
	summary: string
	policy: any
	draft_only?: boolean
	/** No deployed counterpart (draft-only); disables the topbar Diff. */
	no_deployed?: boolean
	custom_path?: string
}

// One editor cell per (kind, path) the session loads: the load slot plus the
// content/baseline stores for that item. Keying by path lets several items of the
// same kind stay loaded — and mounted as separate live editors — at once. A tab's
// editor resolves its own cell via the accessors below.
export interface FlowCell {
	slot: LoadSlot
	store: StateStore<Flow>
	stateStore: { val: Record<string, any> }
	saved: { val: SavedFlow | undefined }
}
export interface ScriptCell {
	slot: LoadSlot
	store: { val: NewScript | undefined }
	saved: { val: SavedScript | undefined }
}
export interface RawAppCell {
	slot: LoadSlot
	store: { val: RawAppRuntimeValue | undefined }
	saved: { val: RawAppSavedValue | undefined }
}

export interface SessionRuntime {
	readonly sessionId: string
	readonly manager: AIChatManager
	// Single live owner of this session's preview tabs. Both the sessions page
	// (renderer) and the open_preview/get_preview_status tools cross it, so the
	// tab model has exactly one live copy.
	readonly previewTabs: SessionPreviewTabs
	// Pipeline target state — persists across editor hide/show (the pane unmounts
	// on hide, so this can't be component-local) and across session switches.
	readonly pipelineEditorState: PipelineEditorState
	// Per-(kind, path) editor cells (content/baseline stores + load slot), created
	// on demand. Each editable preview tab resolves its own cell, so several items
	// stay live at once.
	flowCell(path: string): FlowCell
	loadFlow(workspace: string, path: string, force?: boolean): Promise<void>
	scriptCell(path: string): ScriptCell
	loadScript(workspace: string, path: string, force?: boolean): Promise<void>
	rawAppCell(path: string): RawAppCell
	// Non-creating peek at an editor cell's settled path (undefined when no cell
	// exists yet for this (kind, path)), so callers can check load state without
	// the cell accessors' create-on-miss side effect.
	loadedEditorPath(kind: SessionTargetKind, path: string): string | undefined
	loadRawApp(
		workspace: string,
		path: string,
		force?: boolean,
		deployedOnly?: boolean
	): Promise<void>
	setRuntimeLogRequester(requester: RawAppRuntimeLogRequester | undefined): void
	requestRuntimeLogs(limit: number): Promise<RawAppRuntimeLogEntry[] | undefined>
	/** Register a mounted raw-app preview's DOM requester, keyed by app path.
	 * ALL mounted preview tabs register (hidden ones stay mounted), so a
	 * DOM-scoped turn can read its own app even when another tab is visible. */
	registerDomRequester(appPath: string, requester: RawAppDomRequester): void
	unregisterDomRequester(appPath: string, requester: RawAppDomRequester): void
	/** The visible preview — the default target for a query with no app path. */
	setActiveDomApp(appPath: string, owner: unknown): void
	releaseActiveDomApp(owner: unknown): void
	requestDom(query: RawAppDomQuery): Promise<RawAppDomResult | undefined>
	setAppRunsProvider(provider: RawAppRunsProvider | undefined): void
	getAppRuns(): RawAppRunSummary[] | undefined
	setScreenshotRequester(requester: RawAppScreenshotRequester | undefined): void
	/** Release the slot only if `requester` still owns it. */
	clearScreenshotRequester(requester: RawAppScreenshotRequester): void
	requestScreenshot(): Promise<string | undefined>
	// Discard the local draft + force-reload the editor, so the preview matches
	// the deployed version. Used by editor onDeploy + the chat deploy handler.
	syncPreviewWithDeployed(
		workspace: string,
		kind: 'script' | 'flow' | 'raw_app',
		path: string
	): void
}

const runtimes = new SvelteMap<string, SessionRuntime>()

function emptyFlow(): Flow {
	return {
		summary: '',
		value: { modules: [] },
		path: '',
		edited_at: '',
		edited_by: '',
		archived: false,
		extra_perms: {},
		schema: emptySchema()
	}
}

function emptyLoadSlot(): LoadSlot {
	return { loadedPath: undefined, loadedWorkspace: undefined, loading: false, notFound: false }
}

// Cell factories — a cell starts in the empty-editor state (empty flow / no
// script / no app) until its first load populates it.
function makeFlowCell(): FlowCell {
	const slot: LoadSlot = $state(emptyLoadSlot())
	const store: StateStore<Flow> = $state({ val: emptyFlow() })
	const stateStore: { val: Record<string, any> } = $state({ val: {} })
	const saved: { val: SavedFlow | undefined } = $state({ val: undefined })
	return { slot, store, stateStore, saved }
}
function makeScriptCell(): ScriptCell {
	const slot: LoadSlot = $state(emptyLoadSlot())
	const store: { val: NewScript | undefined } = $state({ val: undefined })
	const saved: { val: SavedScript | undefined } = $state({ val: undefined })
	return { slot, store, saved }
}
function makeRawAppCell(): RawAppCell {
	const slot: LoadSlot = $state(emptyLoadSlot())
	const store: { val: RawAppRuntimeValue | undefined } = $state({ val: undefined })
	const saved: { val: RawAppSavedValue | undefined } = $state({ val: undefined })
	return { slot, store, saved }
}

const GENERATED_SUMMARY_TIMEOUT_MS = 15000
const GENERATED_SUMMARY_MAX_TRANSCRIPT_CHARS = 4000
const GENERATED_SUMMARY_MAX_LENGTH = 60

function buildSummaryTranscript(displayMessages: DisplayMessage[]): string {
	return displayMessages
		.filter((message) => message.role === 'user' || message.role === 'assistant')
		.slice(0, 6)
		.map((message) => `${message.role}: ${message.content}`)
		.join('\n\n')
		.slice(0, GENERATED_SUMMARY_MAX_TRANSCRIPT_CHARS)
}

function normalizeGeneratedSummary(summary: string | undefined): string | undefined {
	const firstLine = summary
		?.split('\n')
		.map((line) => line.trim())
		.find((line) => line.length > 0)
	if (!firstLine) return undefined
	const title = firstLine
		.replace(/^title:\s*/i, '')
		.replace(/^[-*]\s*/, '')
		.replace(/^["'`]+|["'`]+$/g, '')
		.replace(/[.!?]+$/g, '')
		.replace(/\s+/g, ' ')
		.trim()
	if (!/[A-Za-z0-9]/.test(title)) return undefined
	if (title.length <= GENERATED_SUMMARY_MAX_LENGTH) return title
	return title.slice(0, GENERATED_SUMMARY_MAX_LENGTH).trim()
}

async function generateSessionSummary(
	displayMessages: DisplayMessage[]
): Promise<string | undefined> {
	const transcript = buildSummaryTranscript(displayMessages)
	if (!transcript) return undefined
	const abortController = new AbortController()
	const timeout = setTimeout(() => abortController.abort(), GENERATED_SUMMARY_TIMEOUT_MS)
	const messages: ChatCompletionMessageParam[] = [
		{
			role: 'system',
			content:
				'Name this AI chat session. Return only a concise title, 2 to 6 words, no quotes, no period.'
		},
		{
			role: 'user',
			content: `Conversation:\n${transcript}`
		}
	]
	try {
		return normalizeGeneratedSummary(
			await getNonStreamingMetadataCompletion(messages, abortController)
		)
	} finally {
		clearTimeout(timeout)
	}
}

async function generateAndApplySessionSummary(sessionId: string, manager: AIChatManager) {
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	if (!session || session.summarySource !== 'placeholder') return
	const chatId = session.chatId
	if (!chatId) return
	const title = await generateSessionSummary([...manager.displayMessages])
	if (!title) return
	setGeneratedSessionSummary(sessionId, title, chatId)
}

function createRuntime(session: Session): SessionRuntime {
	const manager = new AIChatManager()
	manager.disabledModes = { navigator: true }
	// Sessions always operate in GLOBAL mode (workspace-item tools across
	// the session's workspace). The page-level gate already requires the
	// global-AI flag, so this is always available here. Mode is locked
	// and the dropdown is hidden in the chat UI.
	manager.mode = AIMode.GLOBAL
	// Session chats drive a side-panel preview, so they get the session-only
	// preview tools (open_preview / get_preview_status); the global side-panel
	// chat does not.
	manager.isSessionChat = true
	// Carried into the tool helpers so this session's preview/deploy tool calls
	// dispatch to THIS session even when another session is the UI-active one.
	manager.sessionId = session.id
	// The chat targets the session's OWN (possibly forked) workspace without
	// switching the global workspaceStore. Resolved live from the session record
	// so it tracks the pending → committed (and staged-fork) transitions.
	manager.workspaceResolver = () => {
		const s = sessionState.sessions.find((x) => x.id === session.id)
		return s ? getEffectiveWorkspaceId(s) : undefined
	}
	// Session facts (fork vs live workspace) for the system prompt. A resolver so
	// each rebuild reads the current record — the fork commits at first send, and
	// the user can re-point the session's workspace between sends.
	manager.sessionContextResolver = () => {
		const s = sessionState.sessions.find((x) => x.id === session.id)
		if (!s) return undefined
		const wsId = getEffectiveWorkspaceId(s)
		const ws = get(userWorkspaces).find((w) => w.id === wsId)
		return {
			workspaceId: wsId,
			workspaceName: ws?.name,
			parentWorkspaceId: ws?.parent_workspace_id ?? undefined,
			isDevWorkspace: ws?.is_dev_workspace,
			pendingForkOf: s.pending_fork?.parent_workspace_id
		}
	}
	// Pre-flight: materialise the (still-transient) session, then commit
	// the workspace (creating a staged fork if needed) before any send.
	// AIChatManager awaits this so the first message hits a persisted
	// session targeting the right workspace. Both calls are idempotent.
	manager.beforeSend = async () => {
		materializeTransient(session.id)
		// Session is now persisted → flush any linked files buffered while it was transient.
		await manager.attachedFiles.flushPending()
		// Fork creation is the slow part of the pre-flight; label the loading
		// indicator so the user knows why the send is taking a moment.
		manager.loadingLabel = 'Creating workspace fork...'
		const committed = await commitSessionWorkspace(session.id, get(workspaceStore) ?? undefined)
		manager.loadingLabel = undefined
		// commitSessionWorkspace returns undefined only when the session did NOT
		// commit to a workspace — most importantly when a staged fork failed to
		// materialise (materializeFork is built to toast + return undefined rather
		// than throw). Throwing here is what makes AIChatManager.sendRequest abort:
		// otherwise the send proceeds against get(workspaceStore) (the parent for a
		// staged-new-fork draft), shipping the message + its tool calls to the
		// wrong workspace while the pending_fork is silently dropped.
		if (!committed) {
			throw new Error(
				'the session workspace could not be created or committed (fork creation may have failed)'
			)
		}
		// The composer may have been enabled by a previous workspace's copilot
		// config (copilotInfo is global). getCurrentModel() reads it when the
		// request builds just after this hook, so load the committed workspace's
		// config first — otherwise a send right after switching workspaces could
		// pick the old provider/model while the proxy + tools target the new one.
		// SessionWrapper's active-session load usually already did this; skip the
		// fetch when it matches (a staged fork commits to a fresh id its load missed).
		if (get(copilotWorkspace) !== committed) {
			await loadCopilot(committed)
		}
	}
	manager.afterFirstTurnSaved = () => generateAndApplySessionSummary(session.id, manager)

	// One cell per (kind, path). Created on demand by the load methods; each holds
	// cached content (KB–MB), not a mounted editor. Bounded to the items open
	// preview tabs reference: pruneEditorCells (below) drops the rest when the tab
	// set changes, so re-pointing one tab through many items can't leak cells.
	const flowCells = new Map<string, FlowCell>()
	const scriptCells = new Map<string, ScriptCell>()
	const rawAppCells = new Map<string, RawAppCell>()
	function flowCell(path: string): FlowCell {
		let c = flowCells.get(path)
		if (!c) flowCells.set(path, (c = makeFlowCell()))
		return c
	}
	function scriptCell(path: string): ScriptCell {
		let c = scriptCells.get(path)
		if (!c) scriptCells.set(path, (c = makeScriptCell()))
		return c
	}
	function rawAppCell(path: string): RawAppCell {
		let c = rawAppCells.get(path)
		if (!c) rawAppCells.set(path, (c = makeRawAppCell()))
		return c
	}
	function loadedEditorPath(kind: SessionTargetKind, path: string): string | undefined {
		const cell =
			kind === 'flow'
				? flowCells.get(path)
				: kind === 'script'
					? scriptCells.get(path)
					: rawAppCells.get(path)
		return cell?.slot.loadedPath
	}
	// Drop every editor cell no open preview tab still points at. Called on each
	// tab-set change: a closed or navigated-away item's cell (and its cached
	// content) is reclaimed. Deduping keeps at most one editor tab per item, so an
	// item absent from the open tabs has no live editor to strand.
	function pruneEditorCells(): void {
		const keep = { flow: new Set<string>(), script: new Set<string>(), raw_app: new Set<string>() }
		for (const t of previewTabs.tabs) {
			const route = parsePreviewItemRoute(t.url)
			if (!route) continue
			const kind = route.raw_app ? 'raw_app' : route.kind
			if (kind === 'flow' || kind === 'script' || kind === 'raw_app') keep[kind].add(route.itemPath)
		}
		for (const p of [...flowCells.keys()]) if (!keep.flow.has(p)) flowCells.delete(p)
		for (const p of [...scriptCells.keys()]) if (!keep.script.has(p)) scriptCells.delete(p)
		for (const p of [...rawAppCells.keys()]) if (!keep.raw_app.has(p)) rawAppCells.delete(p)
	}

	// Hydrate the preview-tab owner from the session record (the durable backing);
	// from here on the owner is the single live copy and writes back through the
	// adapter. setSessionTabs / setSessionPreviewCollapsed stay the low-level record
	// writers (opening/moving a tab is a touch that persists an in-memory draft).
	const previewTabs = new SessionPreviewTabs(hydratePreviewTabs(session), {
		persist: (snap) => {
			setSessionTabs(session.id, snap.tabs, snap.activeId)
			setSessionPreviewCollapsed(session.id, snap.collapsed)
			// Only persist a real width; undefined means "never resized" (defaults to 50).
			if (snap.previewSize != null) setSessionPreviewSize(session.id, snap.previewSize)
		},
		onTabsChanged: pruneEditorCells
	})

	// Let the jobs tray open a run in this session's preview panel (as an iframe
	// tab over the run page). The global side-panel chat leaves this unset and
	// falls back to a new browser tab.
	manager.openRunInPreview = ({ jobId, workspace, label }) => {
		previewTabs.open({
			type: 'page',
			href: `${base}/run/${jobId}?workspace=${workspace}`,
			label
		})
	}

	manager.openArtifact = (id, name) => {
		// Capture before open() un-collapses / re-activates: flash only when the tab
		// was already the displayed one (nothing else visibly changes).
		const wasDisplayed = !previewTabs.collapsed
		const prevActive = previewTabs.activeId
		const { status } = previewTabs.open({ type: 'artifact', id, name })
		if (status === 'focused' && wasDisplayed && previewTabs.activeId === prevActive) {
			previewTabs.pulseFocus(previewTabs.activeId)
		}
	}
	manager.closeArtifact = (id) => previewTabs.closeArtifact(id)
	// Key the store before any configureGlobalMode runs, so a new session's first create shows at once.
	void manager.artifacts.setSession(session.id)

	// Pipeline target state lives on the runtime (not the PipelineEditorView
	// component) so the in-session drafts survive hide/show of the editor pane —
	// the pane unmounts on hide, and a component-local store would be discarded.
	const pipelineEditorState = new PipelineEditorState()

	let runtimeLogRequester: RawAppRuntimeLogRequester | undefined = undefined
	// appPath → requester, one entry per mounted raw-app preview tab.
	const domRequesters = new Map<string, RawAppDomRequester>()
	let activeDomAppPath: string | undefined = undefined
	let activeDomOwner: unknown = undefined
	let appRunsProvider: RawAppRunsProvider | undefined = undefined
	let screenshotRequester: RawAppScreenshotRequester | undefined = undefined

	return {
		sessionId: session.id,
		manager,
		previewTabs,
		pipelineEditorState,
		flowCell,
		loadedEditorPath,

		async loadFlow(workspace: string, path: string, force = false) {
			const { slot, store, stateStore, saved } = flowCell(path)
			if (slot.loadedPath === path && slot.loadedWorkspace === workspace && !force) return
			// See loadScript: forced reload remounts via the render gate. A workspace
			// retarget (same path, new fork) drops the stale content the same way so
			// the editor gate shows loading and outbound sync can't write the old
			// workspace's content into the new one before the fetch lands.
			if (force || slot.loadedWorkspace !== workspace) slot.loadedPath = undefined
			slot.loading = true
			slot.notFound = false
			try {
				// Draft first. UserDraft is the shared authoritative content
				// source — the chat (write_flow / patch_flow_json /
				// set_flow_module_code) and the editor's outbound $effect both
				// write through it. If a draft exists we render from it, even
				// when the path has never been deployed.
				const aiDraft = UserDraft.get<Flow>('flow', path, { workspace })

				// getDraft=true omits version_id (the plain getFlowByPath has it) —
				// stamp it on so the flow doesn't always diff. Best-effort.
				let deployedVersionId: number | undefined
				try {
					deployedVersionId = (await FlowService.getFlowByPath({ workspace, path }))?.version_id
				} catch {
					deployedVersionId = undefined
				}

				if (aiDraft) {
					// Best-effort fetch the backend baseline for the diff
					// drawer. Don't fail the load if the path doesn't exist
					// yet on the backend — draft-only flows are a valid state.
					try {
						const result = await FlowService.getFlowByPath({ workspace, path, getDraft: true })
						saved.val = result as SavedFlow
					} catch {
						saved.val = undefined
					}
					await initFlow(aiDraft, store, stateStore, workspace)
					if (deployedVersionId != null && store.val) store.val.version_id = deployedVersionId
					slot.loadedPath = path
					slot.loadedWorkspace = workspace
					return
				}

				// No local draft yet — seed from `result.draft ?? result`.
				const result = await FlowService.getFlowByPath({ workspace, path, getDraft: true })
				saved.val = result as SavedFlow
				const flow: Flow = ((result as SavedFlow).draft ?? (result as Flow)) as Flow
				// Seed the per-tab last_sync from the server draft's timestamp so the
				// seeding save below attaches a matching last_sync and the server can
				// reject stale writes (see loadRawApp). Without this a server draft —
				// saved from the standalone editor or another device — is clobbered
				// with a fresh created_at, so the standalone editor's next autosave
				// (carrying its own last_sync) is rejected as a conflict.
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace, itemKind: 'flow', path },
					(result as SavedFlow).draft_saved_at
				)
				UserDraft.save('flow', path, flow, { workspace })
				await initFlow(flow, store, stateStore, workspace)
				if (deployedVersionId != null && store.val) store.val.version_id = deployedVersionId
				slot.loadedPath = path
				slot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load flow', err)
				slot.notFound = true
			} finally {
				slot.loading = false
			}
		},

		scriptCell,

		async loadScript(workspace: string, path: string, force = false) {
			const { slot, store, saved } = scriptCell(path)
			if (slot.loadedPath === path && slot.loadedWorkspace === workspace && !force) return
			// Forced reload: clearing the slot's loadedPath drops us into
			// SessionEditorTarget's `{:else if slot.loadedPath === undefined}` gate,
			// which unmounts then remounts the editor — avoids the Monaco init race a
			// synchronous {#key} would hit.
			if (force || slot.loadedWorkspace !== workspace) slot.loadedPath = undefined
			slot.loading = true
			slot.notFound = false
			try {
				// Draft first. UserDraft is the shared authoritative content
				// source — the chat (write_script / edit_script) and the
				// editor's outbound $effect both write through it. If a draft
				// exists we render from it, even when the path has never been
				// deployed.
				const aiDraft = UserDraft.get<NewScript>('script', path, { workspace })

				if (aiDraft && typeof aiDraft.content === 'string') {
					// Best-effort fetch the backend baseline for the diff
					// drawer + parent_hash. 404 means draft-only — leave
					// savedScript undefined and skip parent_hash.
					try {
						const result = await ScriptService.getScriptByPath({ workspace, path, getDraft: true })
						saved.val = result as SavedScript
					} catch {
						saved.val = undefined
					}
					// Clone before layering the AI draft on top, else we'd mutate
					// `saved.val` in place and lose the pristine diff baseline.
					const baseline: NewScript = saved.val
						? (structuredClone(
								$state.snapshot(
									(saved.val.draft as NewScript | undefined) ?? (saved.val as NewScript)
								)
							) as NewScript)
						: {
								// Seed from the draft's own path (a rename lives in `draft_path`,
								// else `path`), not the storage key. Otherwise re-seeding a renamed
								// never-deployed draft (e.g. a script→script switch re-runs loadScript
								// with the draft still in memory) resets the path to `draft_<uuid>`,
								// and the next autosave drops `draft_path` — clobbering the rename.
								path:
									(aiDraft as NewScript & { draft_path?: string }).draft_path ??
									aiDraft.path ??
									path,
								summary: aiDraft.summary ?? '',
								content: '',
								description: '',
								schema: emptySchema(),
								language: (aiDraft.language ?? 'bun') as any
							}
					if (saved.val?.hash) {
						baseline.parent_hash = saved.val.hash
					}
					baseline.content = aiDraft.content
					if (aiDraft.language) baseline.language = aiDraft.language
					if (aiDraft.summary !== undefined) baseline.summary = aiDraft.summary
					store.val = baseline
					slot.loadedPath = path
					slot.loadedWorkspace = workspace
					return
				}

				// No local draft yet — seed from `result.draft ?? result`.
				const result = await ScriptService.getScriptByPath({ workspace, path, getDraft: true })
				saved.val = result as SavedScript
				// Clone before mutating, else `baseline` aliases `result` and
				// `baseline.parent_hash` corrupts the diff baseline.
				const baseline = structuredClone(
					((result as SavedScript).draft as NewScript | undefined) ?? (result as NewScript)
				)
				baseline.parent_hash = result.hash
				// Seed the per-tab last_sync from the server draft's timestamp so the
				// seeding save below attaches a matching last_sync and the server can
				// reject stale writes (see loadRawApp). Without this a server draft —
				// saved from the standalone editor or another device — is clobbered
				// with a fresh created_at, so the standalone editor's next autosave
				// (carrying its own last_sync) is rejected as a conflict.
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace, itemKind: 'script', path },
					(result as SavedScript).draft_saved_at
				)
				UserDraft.save<NewScript>('script', path, baseline, { workspace })
				store.val = baseline
				slot.loadedPath = path
				slot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load script', err)
				slot.notFound = true
			} finally {
				slot.loading = false
			}
		},

		rawAppCell,

		async loadRawApp(workspace: string, path: string, force = false, deployedOnly = false) {
			const { slot, store, saved } = rawAppCell(path)
			if (slot.loadedPath === path && slot.loadedWorkspace === workspace && !force) return
			// See loadScript: forced reload remounts via the render gate.
			if (force || slot.loadedWorkspace !== workspace) slot.loadedPath = undefined
			slot.loading = true
			slot.notFound = false
			try {
				// Draft first. UserDraft is the shared authoritative content
				// source — the chat (init_app / write_app_file / ...) and the
				// editor's outbound $effect both write through it. If a draft
				// exists we render from it, even when the path has never been
				// deployed.
				const aiDraft = deployedOnly
					? undefined
					: UserDraft.get<RawAppDraft>('raw_app', path, { workspace })

				if (aiDraft) {
					// Best-effort fetch the backend baseline for the diff
					// drawer. Don't fail the load if the path doesn't exist
					// yet on the backend — draft-only apps are a valid state.
					try {
						const result = await AppService.getAppByPath({
							workspace,
							path,
							getDraft: true,
							rawApp: true
						})
						// Top-level fields are the deployed payload — the diff
						// baseline, since the session has its own `aiDraft`.
						saved.val = {
							summary: result.summary,
							value: result.value as any,
							path: result.path,
							policy: result.policy,
							custom_path: result.custom_path,
							no_deployed: result.no_deployed
						}
					} catch {
						saved.val = undefined
					}
					store.val = applyDraftToRuntimeRawApp(
						{
							files: {},
							runnables: {},
							data: { ...DEFAULT_DATA },
							policy: undefined,
							summary: aiDraft.summary ?? '',
							path
						},
						aiDraft
					)
					slot.loadedPath = path
					slot.loadedWorkspace = workspace
					return
				}

				// No local draft yet — seed from the server draft if present
				// (the standalone editor's "Save draft"), else the deployed value.
				const result = await AppService.getAppByPath({
					workspace,
					path,
					getDraft: !deployedOnly,
					rawApp: true
				})
				// Deployed baseline for the diff drawer (top-level fields).
				saved.val = {
					summary: result.summary,
					value: result.value as any,
					path: result.path,
					policy: result.policy,
					custom_path: result.custom_path,
					no_deployed: result.no_deployed
				}
				// Prefer the server draft over the deployed value (mirrors the
				// flow/script `result.draft ?? result`). A raw-app draft is already
				// editor-shaped, same keys the extraction below reads.
				const draftValue: any = deployedOnly ? undefined : (result as any).draft
				const sourceValue: any = draftValue ?? result.value
				let data: RawAppData = { ...DEFAULT_DATA }
				if (sourceValue?.data) {
					const d = sourceValue.data
					if (d.creation) {
						data = {
							tables: d.tables ?? [],
							datatable: d.creation.datatable,
							schema: d.creation.schema
						}
					} else {
						data = d
					}
				} else if (sourceValue?.datatables) {
					data = { ...DEFAULT_DATA, tables: sourceValue.datatables }
				}
				const runtimeValue = {
					files: (sourceValue?.files ?? {}) as Record<string, string>,
					runnables: (sourceValue?.runnables ?? {}) as Record<string, any>,
					data,
					policy: draftValue?.policy ?? result.policy,
					summary: draftValue?.summary ?? result.summary ?? '',
					path: result.path,
					custom_path: draftValue?.custom_path ?? result.custom_path,
					draft_path: draftValue?.draft_path
				}
				// Seed the per-tab last_sync from the server draft's timestamp so
				// later saves attach a matching last_sync and the server can reject
				// stale writes (a fresh tab has none, so its first POST is treated
				// as fresh and overwrites unconditionally).
				UserDraftDbSyncer.recordRemoteSync(
					{ workspace, itemKind: 'raw_app', path },
					(result as any).draft_saved_at as string | undefined
				)
				UserDraft.save('raw_app', path, runtimeRawAppToDraft(runtimeValue), { workspace })
				store.val = runtimeValue
				slot.loadedPath = path
				slot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load raw app', err)
				slot.notFound = true
			} finally {
				slot.loading = false
			}
		},

		syncPreviewWithDeployed(workspace, kind, path) {
			// After deploy the editor state equals the deployed value; the reload
			// below re-seeds the cell from it, which must NOT POST as a fresh draft.
			// The full-page editor guards this with discardDraftAfterDeploy, but the
			// shared header skips that in a session pane (inSessionPane) and routes
			// post-deploy cleanup here — so apply the same stopSync + arm-restart
			// bracket. Covers all three kinds since they all funnel through this.
			UserDraft.stopSync(kind, path, { workspace })
			UserDraft.discard(kind, path, undefined, { workspace })
			if (kind === 'script') void this.loadScript(workspace, path, true)
			else if (kind === 'flow') void this.loadFlow(workspace, path, true)
			else void this.loadRawApp(workspace, path, true)
			armRestartOnFirstInteraction(workspace, kind, path)
		},

		setRuntimeLogRequester(requester) {
			runtimeLogRequester = requester
		},
		async requestRuntimeLogs(limit) {
			return runtimeLogRequester ? runtimeLogRequester(limit) : undefined
		},
		registerDomRequester(appPath, requester) {
			domRequesters.set(appPath, requester)
		},
		unregisterDomRequester(appPath, requester) {
			// Identity-guarded: a remount may already have replaced this entry.
			if (domRequesters.get(appPath) === requester) domRequesters.delete(appPath)
		},
		setActiveDomApp(appPath, owner) {
			activeDomAppPath = appPath
			activeDomOwner = owner
		},
		releaseActiveDomApp(owner) {
			// Owner-guarded so a set/release race between two tabs can't blank the
			// new active app regardless of effect order.
			if (activeDomOwner === owner) {
				activeDomAppPath = undefined
				activeDomOwner = undefined
			}
		},
		async requestDom(query) {
			if (domRequesters.size === 0) return undefined
			// Route to the query's own app when specified (a DOM-scoped turn reads
			// its element's app even when another tab is now visible), else the
			// active preview, else the only one open.
			const path =
				query.appPath ??
				activeDomAppPath ??
				(domRequesters.size === 1 ? [...domRequesters.keys()][0] : undefined)
			if (path === undefined) return undefined
			const requester = domRequesters.get(path)
			if (!requester) {
				return {
					text: `The preview for "${path}" is no longer open, so its DOM can't be read. Re-open that raw app in the session to inspect it.`
				}
			}
			return requester(query)
		},
		setAppRunsProvider(provider) {
			appRunsProvider = provider
		},
		getAppRuns() {
			return appRunsProvider ? appRunsProvider() : undefined
		},
		setScreenshotRequester(requester) {
			screenshotRequester = requester
		},
		clearScreenshotRequester(requester) {
			// Tabs unmount in any order and several stay mounted at once, so a
			// departing tab must not unregister whichever one now owns the slot.
			if (screenshotRequester === requester) {
				screenshotRequester = undefined
			}
		},
		async requestScreenshot() {
			return screenshotRequester ? screenshotRequester() : undefined
		}
	}
}

async function initRuntime(runtime: SessionRuntime, session: Session) {
	const { manager } = runtime
	await manager.historyManager.init()
	manager.historyManager.setSessionId(session.id)
	// Restore linked files persisted for this session (live handles re-grant on send;
	// snapshots restore directly). Non-transient sessions persist immediately.
	await manager.attachedFiles.restore(session.id, !session.transient)
	await ensureChatIdsSeeded(manager.historyManager)

	// Keep the session record's chatId following the manager's active chat: a
	// "/clear" rotation or a history switch would otherwise leave it pointing at
	// the previous chat, and the compare-page handoff (`from_session`) would
	// preselect the wrong chat's items.
	manager.onChatRotated = (chatId) => setSessionChatId(session.id, chatId)

	if (session.chatId) {
		manager.historyManager.setCurrentChatId(session.chatId)
		await manager.historyManager.tagChatWithSession(session.chatId, session.id)
		await manager.loadPastChat(session.chatId)
		// loadPastChat only seeds the mask when the chat exists in history; a chatId
		// pointing at a chat not yet persisted (no turn saved) would leave it
		// undefined, and the Edits surface would then show every workspace draft.
		// Start tracking so this session is scoped to its own edits from the outset.
		if (manager.modifiedItems === undefined) manager.initModifiedItemsTracking()
	} else {
		// Brand-new session chat: start tracking modified items now (empty mask)
		// so the session bar filters to this chat's changes from the first turn.
		// (loadPastChat handles seeding for the existing-chat branch above.)
		manager.initModifiedItemsTracking()
		setSessionChatId(session.id, manager.historyManager.getCurrentChatId())
	}
}

export function getOrCreateRuntime(session: Session): SessionRuntime {
	let runtime = runtimes.get(session.id)
	if (!runtime) {
		runtime = createRuntime(session)
		runtimes.set(session.id, runtime)
		initRuntime(runtime, session).catch((e) => console.error('Failed to init session runtime', e))
	}
	return runtime
}

export function disposeRuntime(sessionId: string) {
	const runtime = runtimes.get(sessionId)
	if (!runtime) return
	runtime.manager.cancel('runtime disposed')
	runtime.manager.historyManager.close()
	runtimes.delete(sessionId)
}

export function listRuntimes(): SessionRuntime[] {
	return Array.from(runtimes.values())
}

export function getRuntime(sessionId: string): SessionRuntime | undefined {
	return runtimes.get(sessionId)
}

// Point a session's preview at a single seed tab. For re-pointing an existing
// draft session at a new destination ("Open in AI session" / new-session-from-
// page on a reused transient): its previous tabs — persisted with the draft
// and/or held by a live owner — would otherwise keep showing the old target.
// Must write through the live owner when one exists; a bare record write would
// be clobbered by the owner's next flush.
export function resetSessionPreviewTabs(sessionId: string, url: string): void {
	const tabs = [{ id: 'session', url, loc: url }]
	const rt = runtimes.get(sessionId)
	if (rt) {
		rt.previewTabs.reset(tabs, 'session')
	} else {
		setSessionTabs(sessionId, tabs, 'session')
		setSessionPreviewCollapsed(sessionId, false)
	}
}

export type SessionChatStatus =
	| 'idle'
	| 'streaming'
	| 'awaiting-user'
	| 'needs-confirmation'
	| 'draft'
	| 'error'

// Full session teardown: dispose the runtime and remove from sessionState in one
// call. Callers (sidebar / header dropdowns) just invoke this; navigation away
// from a deleted active session is the caller's responsibility.
export function removeSession(sessionId: string): void {
	disposeRuntime(sessionId)
	deleteSessionState(sessionId)
}

// Register the global open_preview tool handler once at module load. It
// dispatches to the *calling* session (sessionId threaded from the tool ctx),
// falling back to the UI-active session only when none was passed — so a
// backgrounded session's tool call opens its OWN preview, not the one the user
// happens to be viewing. Outside a session there is no calling/active id and
// the tool returns a polite error.
setOpenPreviewHandler(({ sessionId: callerSessionId, kind, path }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) {
		return 'Error: no active session to open the preview in.'
	}
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	if (!session) {
		return 'Error: no active session to open the preview in.'
	}
	const target = previewTargetForSessionTarget(kind, path)
	if (!target) {
		return `Error: ${kind} targets cannot be shown in the preview panel.`
	}
	const result = getOrCreateRuntime(session).previewTabs.open(target)
	return result.status === 'focused'
		? `A preview tab is already showing ${kind} "${path}" — focused it.`
		: `Opened ${kind} preview for ${path} in a new tab in the side panel.`
})

// open_page dispatches here to show a workspace page (Runs/Schedules) as a page
// tab in the calling session's preview panel. Returns undefined when there is no
// session so open_page can fall back to browser navigation.
setOpenPagePreviewHandler(({ sessionId: callerSessionId, href, label, newTab }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) return undefined
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	if (!session) return undefined
	const owner = getOrCreateRuntime(session).previewTabs
	// Re-point the tab already showing this page (matched ignoring query/hash) so a
	// filter change updates it in place instead of spawning a duplicate — unless the
	// user asked for a separate tab. open() dedupes on the exact URL, so differing
	// filters would otherwise always open a new tab.
	const targetPage = matchPreviewPage(href)
	if (!newTab && targetPage) {
		const existing = owner.tabs.find(
			(t) => matchPreviewPage(t.loc || t.url)?.path === targetPage.path
		)
		if (existing) {
			owner.select(existing.id)
			owner.navigate({ type: 'page', href, label })
			owner.setCollapsed(false)
			return `Updated the ${label} preview tab with the new filters.`
		}
	}
	const result = owner.open({ type: 'page', href, label })
	return result.status === 'focused'
		? `A preview tab is already showing ${label} — focused it and applied the filters.`
		: `Opened ${label} in a new preview tab in the side panel.`
})

// Companion to the open_preview handler: report the calling session's open
// preview tabs (the panel is multi-tab), so the assistant can avoid re-opening
// a tab already showing the item it just edited.
setGetPreviewStatusHandler((callerSessionId) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) return 'No active session; the preview panel is unavailable.'
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	if (!session) return 'No active session; the preview panel is unavailable.'
	const owner = getOrCreateRuntime(session).previewTabs
	return describePreview(owner.tabs, owner.activeId)
})

// close_page dispatches here to close preview tabs in the calling session's
// panel. `all` clears every tab; otherwise `match` is a case-insensitive
// substring tested against each tab's page label and stripped path.
setClosePreviewTabsHandler(({ sessionId: callerSessionId, all, match }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) return 'No active session; the preview panel is unavailable.'
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	if (!session) return 'No active session; the preview panel is unavailable.'
	const owner = getOrCreateRuntime(session).previewTabs
	if (owner.tabs.length === 0) return 'The preview panel has no open tabs.'

	const labelFor = (t: (typeof owner.tabs)[number]) => previewLocationLabel(t.loc || t.url)
	// Resolve the doomed tabs to ids up front — close() re-indexes on each call.
	const doomed = selectPreviewTabsToClose(owner.tabs, { all, match })
	if (doomed.length === 0) {
		const open = owner.tabs.map(labelFor).join(', ')
		return `No open tab matched "${match}". Open tabs: ${open}.`
	}
	const closedLabels = doomed.map(labelFor)
	for (const t of doomed) owner.close(t.id)
	return `Closed ${closedLabels.length} preview tab${closedLabels.length === 1 ? '' : 's'} (${closedLabels.join(', ')}).`
})

// After a chat deploy, reload the calling session's preview — only if it's open
// showing that exact item.
setDeployedInSessionHandler(({ sessionId: callerSessionId, kind, path }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) return
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = runtimes.get(sessionId)
	if (!session?.workspace_id || !runtime) return
	// Peek without creating a cell: a deploy for an item with no open editor tab
	// must not allocate an empty cell that lingers until the next prune.
	if (runtime.loadedEditorPath(kind, path) !== path) return
	runtime.syncPreviewWithDeployed(session.workspace_id, kind, path)
})

setGetRuntimeLogsHandler(async ({ sessionId: callerSessionId, limit }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	const runtime = sessionId ? runtimes.get(sessionId) : undefined
	if (!runtime) {
		return {
			aiResult:
				'Error: get_app_runtime_logs is only available inside an AI session. Tell the user runtime logs can only be read from a session preview, or switch to a session and open the raw app preview.',
			uiMessage: 'Runtime logs unavailable',
			toolResult: 'Runtime logs unavailable'
		}
	}
	const entries = await runtime.requestRuntimeLogs(limit)
	if (entries === undefined) {
		return {
			aiResult:
				'No runtime logs are available because no raw app preview is running for this session. Next step: call open_preview with kind="raw_app" and the app path, wait for it to load, then call get_app_runtime_logs again. Runtime logs are read live from the running preview and are not persisted.',
			uiMessage: 'Runtime logs unavailable',
			toolResult: 'Runtime logs unavailable'
		}
	}
	if (entries.length === 0) {
		return {
			aiResult:
				'The raw app preview is running, but it has not emitted console logs, uncaught errors, or unhandled rejections yet. If the user reported a failure, reproduce the interaction in the preview, then call get_app_runtime_logs again. For backend.<id>() failures, call list_app_runs and then get_job_logs for the relevant job_id.',
			uiMessage: 'No runtime logs',
			toolResult: 'No runtime logs'
		}
	}
	const limited = entries.slice(-limit)
	return {
		aiResult: formatRuntimeLogsForChat(limited),
		uiMessage: `Read runtime logs`,
		toolResult: formatRuntimeLogsForChat(limited)
	}
})

setGetDomHandler(async ({ sessionId: callerSessionId, query }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	const runtime = sessionId ? runtimes.get(sessionId) : undefined
	if (!runtime) {
		return {
			aiResult:
				'Error: search_dom and read_dom are only available inside an AI session. Tell the user the rendered DOM can only be read from a session preview, or switch to a session and open the raw app preview.',
			uiMessage: 'DOM unavailable',
			toolResult: 'DOM unavailable'
		}
	}
	const result = await runtime.requestDom(query)
	if (result === undefined) {
		return {
			aiResult:
				'No raw app preview is running for this session, so the DOM cannot be read. Next step: call open_preview with kind="raw_app" and the app path, wait for it to load, then call search_dom or read_dom again. The DOM is read live from the running preview.',
			uiMessage: 'DOM unavailable',
			toolResult: 'DOM unavailable'
		}
	}
	return {
		aiResult: result.text,
		uiMessage: query.mode === 'search' ? 'Searched app DOM' : 'Read app DOM',
		toolResult: result.text
	}
})

setListAppRunsHandler(({ sessionId: callerSessionId, limit }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	const runtime = sessionId ? runtimes.get(sessionId) : undefined
	if (!runtime) {
		return {
			aiResult:
				'Error: list_app_runs is only available inside an AI session. Tell the user app runs can only be read from a session preview.',
			uiMessage: 'App runs unavailable',
			toolResult: 'App runs unavailable'
		}
	}
	const runs = runtime.getAppRuns()
	if (runs === undefined) {
		return {
			aiResult:
				'No raw app preview is open for this session, so no backend runs can be listed. Next step: call open_preview with kind="raw_app" and the app path, let the preview load, then call list_app_runs again.',
			uiMessage: 'App runs unavailable',
			toolResult: 'App runs unavailable'
		}
	}
	if (runs.length === 0) {
		return {
			aiResult: 'No backend runnable executions are tracked for this raw app preview yet.',
			uiMessage: 'No app runs',
			toolResult: 'No app runs'
		}
	}
	const limited = limit > 0 ? runs.slice(0, limit) : runs
	return {
		aiResult: formatAppRunsForChat(limited),
		uiMessage: `Fetched app runs`,
		toolResult: formatAppRunsForChat(limited)
	}
})

setScreenshotHandler(async ({ sessionId: callerSessionId }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	const runtime = sessionId ? runtimes.get(sessionId) : undefined
	if (!runtime) {
		return {
			error:
				'Error: take_screenshot is only available inside an AI session. Tell the user screenshots can only be captured from a session preview.',
			uiMessage: 'Screenshot unavailable'
		}
	}
	try {
		const dataUrl = await runtime.requestScreenshot()
		if (dataUrl === undefined) {
			return {
				error:
					'No raw app preview is open for this session, so there is nothing to screenshot. Next step: call open_preview with kind="raw_app" and the app path, wait for it to load, then call take_screenshot again.',
				uiMessage: 'Screenshot unavailable'
			}
		}
		return { dataUrl }
	} catch (e) {
		return {
			error: `Could not capture the app preview: ${e instanceof Error ? e.message : String(e)}`,
			uiMessage: 'Screenshot failed'
		}
	}
})

export function getSessionChatStatus(runtime: SessionRuntime): SessionChatStatus {
	const m = runtime.manager
	if (m.loading) return 'streaming'
	if (m.instructions.trim().length > 0) return 'draft'
	const last = m.displayMessages[m.displayMessages.length - 1]
	if (last?.role === 'tool' && last.needsConfirmation) return 'needs-confirmation'
	if (last?.role === 'user' && last.error) return 'error'
	if (last && (last.role === 'assistant' || last.role === 'tool')) return 'awaiting-user'
	return 'idle'
}
