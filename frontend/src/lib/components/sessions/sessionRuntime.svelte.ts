import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { get } from 'svelte/store'
import { AIChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
import { initFlow } from '$lib/components/flows/flowStore.svelte'
import {
	AppService,
	FlowService,
	ScriptService,
	WorkspaceService,
	type AppWithLastVersion,
	type Flow,
	type NewScript,
	type NewScriptWithDraft,
	type WorkspaceComparison
} from '$lib/gen'
import type { App as AppValue, HiddenRunnable } from '$lib/components/apps/types'
import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { workspaceStore } from '$lib/stores'
import { emptySchema, type StateStore } from '$lib/utils'
import {
	commitSessionWorkspace,
	deleteSession as deleteSessionState,
	ensureChatIdsSeeded,
	setSessionChatId,
	type Session
} from './sessionState.svelte'

export interface SessionRuntime {
	readonly sessionId: string
	readonly manager: AIChatManager
	// Flow target state
	readonly flowStore: StateStore<Flow>
	readonly flowStateStore: { val: Record<string, any> }
	readonly savedFlow: { val: (Flow & { draft?: Flow | undefined }) | undefined }
	readonly loadingFlow: boolean
	readonly notFound: boolean
	readonly loadedPath: string | undefined
	loadFlow(workspace: string, path: string): Promise<void>
	// Script target state (parallel to flow, populated only for script-targeted sessions)
	readonly scriptStore: { val: NewScript | undefined }
	readonly savedScript: { val: NewScriptWithDraft | undefined }
	readonly loadingScript: boolean
	readonly notFoundScript: boolean
	readonly loadedScriptPath: string | undefined
	loadScript(workspace: string, path: string): Promise<void>
	// App (regular drag-and-drop apps) target state
	readonly appStore: {
		val: (AppWithLastVersion & { draft_only?: boolean; value: any }) | undefined
	}
	readonly savedApp: {
		val:
			| {
					value: AppValue
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
	}
	readonly loadingApp: boolean
	readonly notFoundApp: boolean
	readonly loadedAppPath: string | undefined
	loadApp(workspace: string, path: string): Promise<void>
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
					custom_path?: string
			  }
			| undefined
	}
	readonly loadingRawApp: boolean
	readonly notFoundRawApp: boolean
	readonly loadedRawAppPath: string | undefined
	loadRawApp(workspace: string, path: string): Promise<void>
	// Fork comparison cache: shared between SessionForkBar (count + dropdown)
	// and any future consumer that needs the parent ↔ fork diff list. Keyed
	// implicitly by the (parent, fork) pair last passed to ensureForkComparison;
	// invalidateForkComparison() forces a refresh after a known-mutating action.
	readonly forkComparison: { val: WorkspaceComparison | undefined }
	readonly loadingForkComparison: boolean
	ensureForkComparison(parent: string, fork: string): Promise<void>
	invalidateForkComparison(): void
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

function createRuntime(session: Session): SessionRuntime {
	const manager = new AIChatManager()
	manager.disabledModes = { navigator: true }
	// Sessions always operate in GLOBAL mode (workspace-item tools across
	// the session's workspace). The page-level gate already requires the
	// global-AI flag, so this is always available here. Mode is locked
	// and the dropdown is hidden in the chat UI.
	manager.mode = AIMode.GLOBAL
	// Pre-flight: commit the session's workspace (materialising a staged
	// fork if needed) before any send. AIChatManager awaits this so the
	// first message targets the freshly-committed workspace, not the
	// parent. Idempotent — bails out once workspace_id is set.
	manager.beforeSend = async () => {
		await commitSessionWorkspace(session.id, get(workspaceStore) ?? undefined)
	}

	const flowStore: StateStore<Flow> = $state({ val: emptyFlow() })
	const flowStateStore: { val: Record<string, any> } = $state({ val: {} })
	const savedFlow: { val: (Flow & { draft?: Flow | undefined }) | undefined } = $state({
		val: undefined
	})

	let loadingFlow = $state(false)
	let notFound = $state(false)
	let loadedPath = $state<string | undefined>(undefined)

	const scriptStore: { val: NewScript | undefined } = $state({ val: undefined })
	const savedScript: { val: NewScriptWithDraft | undefined } = $state({ val: undefined })
	let loadingScript = $state(false)
	let notFoundScript = $state(false)
	let loadedScriptPath = $state<string | undefined>(undefined)

	const appStore: {
		val: (AppWithLastVersion & { draft_only?: boolean; value: any }) | undefined
	} = $state({ val: undefined })
	const savedApp: { val: SessionRuntime['savedApp']['val'] } = $state({ val: undefined })
	let loadingApp = $state(false)
	let notFoundApp = $state(false)
	let loadedAppPath = $state<string | undefined>(undefined)

	const rawApp: { val: SessionRuntime['rawApp']['val'] } = $state({ val: undefined })
	const savedRawApp: { val: SessionRuntime['savedRawApp']['val'] } = $state({ val: undefined })
	let loadingRawApp = $state(false)
	let notFoundRawApp = $state(false)
	let loadedRawAppPath = $state<string | undefined>(undefined)

	const forkComparison: { val: WorkspaceComparison | undefined } = $state({ val: undefined })
	let loadingForkComparison = $state(false)
	let forkComparisonKey: string | undefined = undefined

	return {
		sessionId: session.id,
		manager,
		flowStore,
		flowStateStore,
		savedFlow,
		get loadingFlow() {
			return loadingFlow
		},
		get notFound() {
			return notFound
		},
		get loadedPath() {
			return loadedPath
		},

		async loadFlow(workspace: string, path: string) {
			if (loadedPath === path) return
			loadingFlow = true
			notFound = false
			try {
				const result = await FlowService.getFlowByPathWithDraft({ workspace, path })
				savedFlow.val = result
				const flow: Flow = (result.draft as Flow | undefined) ?? (result as Flow)
				await initFlow(flow, flowStore, flowStateStore)
				loadedPath = path
			} catch (err) {
				console.error('Failed to load flow', err)
				notFound = true
			} finally {
				loadingFlow = false
			}
		},

		scriptStore,
		savedScript,
		get loadingScript() {
			return loadingScript
		},
		get notFoundScript() {
			return notFoundScript
		},
		get loadedScriptPath() {
			return loadedScriptPath
		},

		async loadScript(workspace: string, path: string) {
			if (loadedScriptPath === path) return
			loadingScript = true
			notFoundScript = false
			try {
				const result = await ScriptService.getScriptByPathWithDraft({ workspace, path })
				savedScript.val = result
				// Prefer the draft if present, falling back to the deployed version
				scriptStore.val = (result.draft as NewScript | undefined) ?? (result as NewScript)
				if (scriptStore.val) scriptStore.val.parent_hash = result.hash
				loadedScriptPath = path
			} catch (err) {
				console.error('Failed to load script', err)
				notFoundScript = true
			} finally {
				loadingScript = false
			}
		},

		appStore,
		savedApp,
		get loadingApp() {
			return loadingApp
		},
		get notFoundApp() {
			return notFoundApp
		},
		get loadedAppPath() {
			return loadedAppPath
		},

		async loadApp(workspace: string, path: string) {
			if (loadedAppPath === path) return
			loadingApp = true
			notFoundApp = false
			try {
				const result = await AppService.getAppByPathWithDraft({ workspace, path })
				savedApp.val = {
					summary: result.summary,
					value: result.value as AppValue,
					path: result.path,
					policy: result.policy,
					draft_only: result.draft_only,
					draft:
						result.draft?.['summary'] !== undefined
							? result.draft
							: result.draft
								? {
										summary: result.summary,
										value: result.draft,
										path: result.path,
										policy: result.policy,
										custom_path: result.custom_path
									}
								: undefined,
					custom_path: result.custom_path
				}
				if (result.draft) {
					appStore.val =
						result.summary !== undefined
							? { ...result, ...(result.draft as Record<string, any>) }
							: ({ ...result, value: result.draft as any } as any)
				} else {
					appStore.val = result as any
				}
				loadedAppPath = path
			} catch (err) {
				console.error('Failed to load app', err)
				notFoundApp = true
			} finally {
				loadingApp = false
			}
		},

		rawApp,
		savedRawApp,
		get loadingRawApp() {
			return loadingRawApp
		},
		get notFoundRawApp() {
			return notFoundRawApp
		},
		get loadedRawAppPath() {
			return loadedRawAppPath
		},

		async loadRawApp(workspace: string, path: string) {
			if (loadedRawAppPath === path) return
			loadingRawApp = true
			notFoundRawApp = false
			try {
				const result = await AppService.getAppByPathWithDraft({ workspace, path })
				savedRawApp.val = {
					summary: result.summary,
					value: result.value as any,
					path: result.path,
					policy: result.policy,
					draft_only: result.draft_only,
					draft: result.draft,
					custom_path: result.custom_path
				}
				const sourceValue: any = result.draft ?? result.value
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
				rawApp.val = {
					files: (sourceValue?.files ?? {}) as Record<string, string>,
					runnables: (sourceValue?.runnables ?? {}) as Record<string, any>,
					data,
					policy: result.policy,
					summary: result.summary ?? '',
					path: result.path
				}
				loadedRawAppPath = path
			} catch (err) {
				console.error('Failed to load raw app', err)
				notFoundRawApp = true
			} finally {
				loadingRawApp = false
			}
		},

		forkComparison,
		get loadingForkComparison() {
			return loadingForkComparison
		},

		async ensureForkComparison(parent: string, fork: string) {
			const key = `${parent}|${fork}`
			if (forkComparisonKey === key && forkComparison.val) return
			if (loadingForkComparison && forkComparisonKey === key) return
			forkComparisonKey = key
			loadingForkComparison = true
			try {
				forkComparison.val = await WorkspaceService.compareWorkspaces({
					workspace: parent,
					targetWorkspaceId: fork
				})
			} catch (e) {
				console.error('SessionRuntime: forkComparison fetch failed', e)
				forkComparison.val = undefined
				// On error, clear the key so the next call retries.
				if (forkComparisonKey === key) forkComparisonKey = undefined
			} finally {
				loadingForkComparison = false
			}
		},

		invalidateForkComparison() {
			forkComparisonKey = undefined
			forkComparison.val = undefined
		}
	}
}

async function initRuntime(runtime: SessionRuntime, session: Session) {
	const { manager } = runtime
	await manager.historyManager.init()
	manager.historyManager.setSessionId(session.id)
	await ensureChatIdsSeeded(manager.historyManager)

	if (session.chatId) {
		manager.historyManager.setCurrentChatId(session.chatId)
		await manager.historyManager.tagChatWithSession(session.chatId, session.id)
		await manager.loadPastChat(session.chatId)
	} else {
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
