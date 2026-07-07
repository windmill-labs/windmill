import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { get } from 'svelte/store'
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
import { workspaceStore } from '$lib/stores'
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
	setSessionTabs,
	setSessionTarget,
	type Session
} from './sessionState.svelte'
import {
	SessionPreviewTabs,
	describePreview,
	hydratePreviewTabs,
	previewTargetForSessionTarget
} from './sessionPreviewTabs.svelte'
import { UserDraft } from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { armRestartOnFirstInteraction } from '$lib/userDraftToast'
import { applyDraftToRuntimeRawApp, runtimeRawAppToDraft, type RawAppDraft } from './appDraftCodec'
import {
	setDeployedInSessionHandler,
	setGetPreviewStatusHandler,
	setGetRuntimeLogsHandler,
	setListAppRunsHandler,
	setOpenPreviewHandler
} from '$lib/components/copilot/chat/global/core'
import {
	formatRuntimeLogsForChat,
	formatAppRunsForChat,
	type RawAppRuntimeLogEntry,
	type RawAppRuntimeLogRequester,
	type RawAppRunSummary,
	type RawAppRunsProvider
} from '$lib/components/raw_apps/utils'
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
	// Kind-agnostic accessor over the per-kind load slots, for consumers (the
	// editor-target gate) that only need load state and not the typed store.
	slot(kind: SessionTargetKind): LoadSlot
	// Flow target state
	readonly flowStore: StateStore<Flow>
	readonly flowStateStore: { val: Record<string, any> }
	readonly savedFlow: { val: SavedFlow | undefined }
	loadFlow(workspace: string, path: string, force?: boolean): Promise<void>
	// Script target state (parallel to flow, populated only for script-targeted sessions)
	readonly scriptStore: { val: NewScript | undefined }
	readonly savedScript: { val: SavedScript | undefined }
	loadScript(workspace: string, path: string, force?: boolean): Promise<void>
	// Note: legacy drag-and-drop apps are intentionally NOT hosted in the
	// session preview pane (only code-based raw apps are), so there's no
	// app target state here.
	// Raw App (HTML-based) target state
	readonly rawApp: {
		val:
			| {
					files: Record<string, string>
					runnables: Record<string, any>
					data: RawAppData
					policy: any
					summary: string
					path: string
					custom_path?: string
					draft_path?: string
			  }
			| undefined
	}
	readonly savedRawApp: {
		val:
			| {
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
			| undefined
	}
	loadRawApp(
		workspace: string,
		path: string,
		force?: boolean,
		deployedOnly?: boolean
	): Promise<void>
	setRuntimeLogRequester(requester: RawAppRuntimeLogRequester | undefined): void
	requestRuntimeLogs(limit: number): Promise<RawAppRuntimeLogEntry[] | undefined>
	setAppRunsProvider(provider: RawAppRunsProvider | undefined): void
	getAppRuns(): RawAppRunSummary[] | undefined
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
	// Pre-flight: materialise the (still-transient) session, then commit
	// the workspace (creating a staged fork if needed) before any send.
	// AIChatManager awaits this so the first message hits a persisted
	// session targeting the right workspace. Both calls are idempotent.
	manager.beforeSend = async () => {
		materializeTransient(session.id)
		// Session is now persisted → flush any linked files buffered while it was transient.
		await manager.attachedFiles.flushPending()
		const committed = await commitSessionWorkspace(session.id, get(workspaceStore) ?? undefined)
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

	const flowStore: StateStore<Flow> = $state({ val: emptyFlow() })
	const flowStateStore: { val: Record<string, any> } = $state({ val: {} })
	const savedFlow: { val: SavedFlow | undefined } = $state({
		val: undefined
	})

	const flowSlot: LoadSlot = $state({
		loadedPath: undefined,
		loadedWorkspace: undefined,
		loading: false,
		notFound: false
	})

	const scriptStore: { val: NewScript | undefined } = $state({ val: undefined })
	const savedScript: { val: SavedScript | undefined } = $state({ val: undefined })
	const scriptSlot: LoadSlot = $state({
		loadedPath: undefined,
		loadedWorkspace: undefined,
		loading: false,
		notFound: false
	})

	const rawApp: { val: SessionRuntime['rawApp']['val'] } = $state({ val: undefined })
	const savedRawApp: { val: SessionRuntime['savedRawApp']['val'] } = $state({ val: undefined })
	const rawAppSlot: LoadSlot = $state({
		loadedPath: undefined,
		loadedWorkspace: undefined,
		loading: false,
		notFound: false
	})

	// Hydrate the preview-tab owner from the session record (the durable backing);
	// from here on the owner is the single live copy and writes back through the
	// adapter. setSessionTabs / setSessionPreviewCollapsed / setSessionTarget stay
	// the low-level record writers (a transient session's writes land in the
	// localStorage draft slot until it materialises).
	const previewTabs = new SessionPreviewTabs(hydratePreviewTabs(session), {
		persist: (snap) => {
			setSessionTabs(session.id, snap.tabs, snap.activeId)
			setSessionPreviewCollapsed(session.id, snap.collapsed)
		},
		setTarget: (target) => setSessionTarget(session.id, target)
	})

	// Pipeline target state lives on the runtime (not the PipelineEditorView
	// component) so the in-session drafts survive hide/show of the editor pane —
	// the pane unmounts on hide, and a component-local store would be discarded.
	const pipelineEditorState = new PipelineEditorState()

	let runtimeLogRequester: RawAppRuntimeLogRequester | undefined = undefined
	let appRunsProvider: RawAppRunsProvider | undefined = undefined

	return {
		sessionId: session.id,
		manager,
		previewTabs,
		slot(kind: SessionTargetKind): LoadSlot {
			return kind === 'flow' ? flowSlot : kind === 'script' ? scriptSlot : rawAppSlot
		},
		pipelineEditorState,
		flowStore,
		flowStateStore,
		savedFlow,

		async loadFlow(workspace: string, path: string, force = false) {
			if (flowSlot.loadedPath === path && flowSlot.loadedWorkspace === workspace && !force) return
			// See loadScript: forced reload remounts via the render gate. A workspace
			// retarget (same path, new fork) drops the stale content the same way so
			// the editor gate shows loading and outbound sync can't write the old
			// workspace's content into the new one before the fetch lands.
			if (force || flowSlot.loadedWorkspace !== workspace) flowSlot.loadedPath = undefined
			flowSlot.loading = true
			flowSlot.notFound = false
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
						savedFlow.val = result as SavedFlow
					} catch {
						savedFlow.val = undefined
					}
					await initFlow(aiDraft, flowStore, flowStateStore)
					if (deployedVersionId != null && flowStore.val)
						flowStore.val.version_id = deployedVersionId
					flowSlot.loadedPath = path
					flowSlot.loadedWorkspace = workspace
					return
				}

				// No local draft yet — seed from `result.draft ?? result`.
				const result = await FlowService.getFlowByPath({ workspace, path, getDraft: true })
				savedFlow.val = result as SavedFlow
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
				await initFlow(flow, flowStore, flowStateStore)
				if (deployedVersionId != null && flowStore.val) flowStore.val.version_id = deployedVersionId
				flowSlot.loadedPath = path
				flowSlot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load flow', err)
				flowSlot.notFound = true
			} finally {
				flowSlot.loading = false
			}
		},

		scriptStore,
		savedScript,

		async loadScript(workspace: string, path: string, force = false) {
			if (scriptSlot.loadedPath === path && scriptSlot.loadedWorkspace === workspace && !force)
				return
			// Forced reload: clearing the slot's loadedPath drops us into
			// SessionEditorTarget's `{:else if slot.loadedPath === undefined}` gate,
			// which unmounts then remounts the editor — avoids the Monaco init race a
			// synchronous {#key} would hit.
			if (force || scriptSlot.loadedWorkspace !== workspace) scriptSlot.loadedPath = undefined
			scriptSlot.loading = true
			scriptSlot.notFound = false
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
						savedScript.val = result as SavedScript
					} catch {
						savedScript.val = undefined
					}
					// Clone before layering the AI draft on top, else we'd mutate
					// `savedScript.val` in place and lose the pristine diff baseline.
					const baseline: NewScript = savedScript.val
						? (structuredClone(
								$state.snapshot(
									(savedScript.val.draft as NewScript | undefined) ?? (savedScript.val as NewScript)
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
					if (savedScript.val?.hash) {
						baseline.parent_hash = savedScript.val.hash
					}
					baseline.content = aiDraft.content
					if (aiDraft.language) baseline.language = aiDraft.language
					if (aiDraft.summary !== undefined) baseline.summary = aiDraft.summary
					scriptStore.val = baseline
					scriptSlot.loadedPath = path
					scriptSlot.loadedWorkspace = workspace
					return
				}

				// No local draft yet — seed from `result.draft ?? result`.
				const result = await ScriptService.getScriptByPath({ workspace, path, getDraft: true })
				savedScript.val = result as SavedScript
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
				scriptStore.val = baseline
				scriptSlot.loadedPath = path
				scriptSlot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load script', err)
				scriptSlot.notFound = true
			} finally {
				scriptSlot.loading = false
			}
		},

		rawApp,
		savedRawApp,

		async loadRawApp(workspace: string, path: string, force = false, deployedOnly = false) {
			if (rawAppSlot.loadedPath === path && rawAppSlot.loadedWorkspace === workspace && !force)
				return
			// See loadScript: forced reload remounts via the render gate.
			if (force || rawAppSlot.loadedWorkspace !== workspace) rawAppSlot.loadedPath = undefined
			rawAppSlot.loading = true
			rawAppSlot.notFound = false
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
						savedRawApp.val = {
							summary: result.summary,
							value: result.value as any,
							path: result.path,
							policy: result.policy,
							custom_path: result.custom_path,
							no_deployed: result.no_deployed
						}
					} catch {
						savedRawApp.val = undefined
					}
					rawApp.val = applyDraftToRuntimeRawApp(
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
					rawAppSlot.loadedPath = path
					rawAppSlot.loadedWorkspace = workspace
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
				savedRawApp.val = {
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
				rawApp.val = runtimeValue
				rawAppSlot.loadedPath = path
				rawAppSlot.loadedWorkspace = workspace
			} catch (err) {
				console.error('Failed to load raw app', err)
				rawAppSlot.notFound = true
			} finally {
				rawAppSlot.loading = false
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
		setAppRunsProvider(provider) {
			appRunsProvider = provider
		},
		getAppRuns() {
			return appRunsProvider ? appRunsProvider() : undefined
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

// MRU set of session ids whose FlowEditorView is currently mounted. Capped at
// MAX_WARM_EDITORS — sessions outside the set show chat-only. Module-scoped so
// both the page (which mutates) and the sidebar (which reads for the dev clue)
// see the same state.
const MAX_WARM_EDITORS = 3
export const editorWarmIds = new SvelteSet<string>()

// Full session teardown: dispose the runtime, drop the LRU entry, and remove
// from sessionState in one call. Callers (sidebar / header dropdowns) just
// invoke this; navigation away from a deleted active session is the caller's
// responsibility.
export function removeSession(sessionId: string): void {
	disposeRuntime(sessionId)
	editorWarmIds.delete(sessionId)
	deleteSessionState(sessionId)
}

export function promoteEditorWarm(sessionId: string): void {
	editorWarmIds.delete(sessionId)
	editorWarmIds.add(sessionId)
	while (editorWarmIds.size > MAX_WARM_EDITORS) {
		const oldest = editorWarmIds.values().next().value
		if (oldest === undefined) break
		editorWarmIds.delete(oldest)
	}
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
	promoteEditorWarm(sessionId)
	return result.status === 'focused'
		? `A preview tab is already showing ${kind} "${path}" — focused it.`
		: `Opened ${kind} preview for ${path} in a new tab in the side panel.`
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
	return describePreview(owner.tabs, owner.activeId, session.target)
})

// After a chat deploy, reload the calling session's preview — only if it's open
// showing that exact item.
setDeployedInSessionHandler(({ sessionId: callerSessionId, kind, path }) => {
	const sessionId = callerSessionId ?? sessionState.currentSessionId
	if (!sessionId) return
	const session = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = runtimes.get(sessionId)
	if (!session?.workspace_id || !runtime) return
	if (runtime.slot(kind).loadedPath !== path) return
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
