import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { createLongHash } from '$lib/editorLangUtils'
import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
import { switchWorkspace } from '$lib/storeUtils'

// Switch the global workspace iff the target differs from the active one
// and is non-empty. Centralises the "session needs its workspace in focus"
// rule so picker, deep-link, and workspace-bar paths agree on the same
// semantic. No-op for `undefined` / empty.
export function syncWorkspaceTo(workspaceId: string | undefined): void {
	if (!workspaceId) return
	if (workspaceId === get(workspaceStore)) return
	switchWorkspace(workspaceId)
}
import { WorkspaceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type HistoryManager from '$lib/components/copilot/chat/HistoryManager.svelte'

export type SessionTarget = { kind: 'flow' | 'script' | 'app' | 'raw_app'; path: string }

// Kinds the in-session editor pane can host. Useful for filtering
// dropdowns / pickers to "items the side panel can open".
export const EDITOR_TARGET_KINDS: ReadonlySet<SessionTarget['kind']> = new Set([
	'flow',
	'script',
	'app',
	'raw_app'
])

export type PendingFork = {
	// Existing workspace to fork from (drives routing/scope pre-send).
	parent_workspace_id: string
	// Slug the new fork will use, e.g. `wm-fork-foo`.
	id: string
	// Display name shown in the workspace bar.
	name: string
}

export type Session = {
	id: string
	name: string
	// Committed strictly at first user-message send. Undefined for drafts
	// that have never been sent — those scope by `pending_workspace_id`
	// instead and don't show the fork bar.
	workspace_id?: string
	// Pre-send draft workspace, picked via SessionWorkspaceBar. Drives
	// scope/editor/display while workspace_id is undefined; gets copied
	// into workspace_id at first send and then becomes irrelevant.
	pending_workspace_id?: string
	// Pre-send intent to create a new fork. The actual API call is
	// deferred to first send (via commitSessionWorkspace) so cancelling
	// the draft doesn't leave an orphan fork behind.
	pending_fork?: PendingFork
	chatId?: string
	target?: SessionTarget
	summary?: string
	createdAt: number
}

const STORAGE_KEY = 'windmill_sessions'

const now = Date.now()
const defaultSessions: Session[] = [
	{
		id: createLongHash(),
		name: 'session-1',
		summary: 'testing_flow',
		target: { kind: 'flow', path: 'u/guilhempw/testing_flow' },
		createdAt: now
	},
	{
		id: createLongHash(),
		name: 'session-2',
		summary: 'demo_groups',
		target: { kind: 'flow', path: 'u/guilhempw/demo_groups' },
		createdAt: now
	}
]

function loadSessions(): Session[] {
	if (!BROWSER) return defaultSessions
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		if (raw) {
			const parsed = JSON.parse(raw)
			if (Array.isArray(parsed) && parsed.length > 0) {
				// Drop empty-string workspace_id (older sessions used '' as a
				// missing-value marker) so the undefined-until-first-send invariant
				// holds for legacy drafts. Also migrate the deprecated
				// 'rawapp' target.kind to the canonical 'raw_app'.
				let mutated = false
				for (const s of parsed) {
					if (s.workspace_id === '') {
						delete s.workspace_id
						mutated = true
					}
					if (s.target?.kind === 'rawapp') {
						s.target.kind = 'raw_app'
						mutated = true
					}
				}
				if (mutated) {
					try {
						localStorage.setItem(STORAGE_KEY, JSON.stringify(parsed))
					} catch (e) {
						console.error('Failed to persist normalised sessions', e)
					}
				}
				return parsed as Session[]
			}
		}
	} catch (e) {
		console.error('Failed to load sessions from localStorage', e)
	}
	return defaultSessions
}

export const sessionState = $state<{
	sessions: Session[]
	currentSessionId: string | undefined
}>({
	sessions: loadSessions(),
	currentSessionId: undefined
})

export function persistSessions() {
	if (!BROWSER) return
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify($state.snapshot(sessionState.sessions)))
	} catch (e) {
		console.error('Failed to persist sessions', e)
	}
}

export function findSessionByName(name: string): Session | undefined {
	return sessionState.sessions.find((s) => s.name === name)
}

export function createSession(): Session {
	const existingNumbers = sessionState.sessions
		.map((s) => /^session-(\d+)$/.exec(s.name)?.[1])
		.map((n) => (n ? parseInt(n, 10) : 0))
	const next = (existingNumbers.length ? Math.max(...existingNumbers) : 0) + 1
	const pending = get(workspaceStore)
	const session: Session = {
		id: createLongHash(),
		name: `session-${next}`,
		pending_workspace_id: pending && pending.length > 0 ? pending : undefined,
		createdAt: Date.now()
	}
	sessionState.sessions = [session, ...sessionState.sessions]
	sessionState.currentSessionId = session.id
	persistSessions()
	return session
}

export function setSessionPendingWorkspace(id: string, workspace_id: string) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	const changed = s.pending_workspace_id !== workspace_id || s.pending_fork !== undefined
	s.pending_workspace_id = workspace_id
	// Picking an existing workspace cancels any pending fork intent.
	s.pending_fork = undefined
	if (changed) persistSessions()
}

// Records the user's intent to create a new fork without firing the API
// call yet. Routing/scope stay on the parent workspace until commit.
export function setSessionPendingFork(id: string, fork: PendingFork) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.pending_fork = { ...fork }
	s.pending_workspace_id = fork.parent_workspace_id
	persistSessions()
}

// One-shot commit: locks in workspace_id at first user-message send.
// If a pending fork is set, materialises it via the API first, then
// switches the global workspace to the freshly created fork. Falls back
// to the pending pick, then the active workspace. Clears pending so it
// doesn't shadow later reads.
export async function commitSessionWorkspace(
	id: string,
	fallback?: string
): Promise<string | undefined> {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return undefined
	if (s.workspace_id) return s.workspace_id

	if (s.pending_fork) {
		const fork = s.pending_fork
		try {
			await WorkspaceService.createWorkspaceFork({
				workspace: fork.parent_workspace_id,
				requestBody: { id: fork.id, name: fork.name }
			})
			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			if (get(workspaceStore) !== fork.id) switchWorkspace(fork.id)
			s.workspace_id = fork.id
			s.pending_fork = undefined
			s.pending_workspace_id = undefined
			persistSessions()
			sendUserToast(`Created fork ${fork.name}`)
			return fork.id
		} catch (e: any) {
			sendUserToast(`Could not create fork: ${e?.body ?? e?.message ?? e}`, true)
			return undefined
		}
	}

	const ws = s.pending_workspace_id ?? fallback
	if (!ws) return undefined
	s.workspace_id = ws
	s.pending_workspace_id = undefined
	persistSessions()
	return ws
}

// Effective workspace for scope/routing — committed if set, otherwise the
// pre-send pending pick (which defaults to the workspace at create time).
// Pending forks route via their parent until creation lands.
export function getEffectiveWorkspaceId(session: Session): string | undefined {
	return session.workspace_id ?? session.pending_workspace_id
}

// Canonical mutation for session.target. Persists, optionally seeds the
// session summary, and centralises the path so callers don't reach into
// session.target directly.
export function setSessionTarget(id: string, target: SessionTarget, summary?: string): void {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.target = target
	if (!s.summary && summary) s.summary = summary
	persistSessions()
}

export function selectSession(id: string) {
	sessionState.currentSessionId = id
}

export function renameSession(id: string, newSummary: string) {
	const trimmed = newSummary.trim()
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.summary = trimmed.length > 0 ? trimmed : undefined
	persistSessions()
}

export function deleteSession(id: string) {
	const idx = sessionState.sessions.findIndex((s) => s.id === id)
	if (idx < 0) return
	sessionState.sessions = sessionState.sessions.filter((s) => s.id !== id)
	if (sessionState.currentSessionId === id) {
		sessionState.currentSessionId = sessionState.sessions[0]?.id
	}
	persistSessions()
}

export function setSessionChatId(sessionId: string, chatId: string) {
	const s = sessionState.sessions.find((x) => x.id === sessionId)
	if (s && s.chatId !== chatId) {
		s.chatId = chatId
		persistSessions()
	}
}

let seedPromise: Promise<void> | undefined

// One-shot pairing of the user's two most-recently-modified saved chats with
// the two seeded sessions. Idempotent across all callers / SessionWrappers.
export function ensureChatIdsSeeded(historyManager: HistoryManager): Promise<void> {
	if (!seedPromise) {
		seedPromise = (async () => {
			try {
				await historyManager.init()
				// Read directly from storage so we see chats regardless of this manager's
				// own session-scope filter (getPastChats would hide already-tagged ones).
				const pastChats = historyManager.getAllSavedChats()
				const untagged = pastChats
					.filter((c) => !c.sessionId)
					.sort((a, b) => b.lastModified - a.lastModified)
				let mutated = false
				for (let i = 0; i < Math.min(sessionState.sessions.length, untagged.length); i++) {
					if (!sessionState.sessions[i].chatId) {
						const chatId = untagged[i].id
						const sessionId = sessionState.sessions[i].id
						sessionState.sessions[i].chatId = chatId
						await historyManager.tagChatWithSession(chatId, sessionId)
						mutated = true
					}
				}
				if (mutated) persistSessions()
			} catch (e) {
				console.error('Failed to seed chat ids from history', e)
			}
		})()
	}
	return seedPromise
}
