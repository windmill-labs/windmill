import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { createLongHash } from '$lib/editorLangUtils'
import { random_adj } from '$lib/components/random_positive_adjetive'
import {
	userWorkspaces,
	usersWorkspaceStore,
	workspaceStore,
	type UserWorkspace
} from '$lib/stores'
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
import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type HistoryManager from '$lib/components/copilot/chat/HistoryManager.svelte'

// Kinds the in-session editor pane can host. Legacy drag-and-drop apps are
// intentionally not previewable — only code-based 'raw_app' apps are.
export type SessionTarget = { kind: 'flow' | 'script' | 'raw_app'; path: string }

// Useful for filtering dropdowns / pickers to "items the side panel can open".
export const EDITOR_TARGET_KINDS: ReadonlySet<SessionTarget['kind']> = new Set([
	'flow',
	'script',
	'raw_app'
])

// Lifecycle status for a fork session. Git-parallel:
//   in_sync     — fork is up to date with parent (or only behind — treated
//                 the same since the user has no unmerged work either way).
//   ahead       — fork has unmerged changes vs parent (branch ahead).
//   diverged    — fork has unmerged changes AND parent has moved (branch
//                 diverged from upstream — potential conflicts).
//   unavailable — fork workspace is no longer in the user's list (deleted,
//                 archived, or access revoked). Read-only fallback.
//
// `undefined` is the loading / not-applicable state (root session,
// comparison not yet fetched).
export type ForkStatus = 'in_sync' | 'ahead' | 'diverged' | 'unavailable'

// Whether the session points at a workspace that is itself a fork (i.e.
// has a parent). Independent of comparison-fetch state — used by the
// sidebar to pick between a root (Building) icon and a fork-status icon
// before the comparison has loaded.
//
// Sessions whose committed workspace is no longer in the user's list are
// still treated as forks (the "unavailable" terminal state) so we don't
// flip them back to Building once access is lost.
export function isForkSession(session: Session, allWorkspaces: UserWorkspace[]): boolean {
	const wsId = session.workspace_id ?? session.pending_workspace_id
	if (!wsId) return false
	const ws = allWorkspaces.find((w) => w.id === wsId)
	if (!ws) return !!session.workspace_id
	return !!ws.parent_workspace_id
}

export function deriveForkStatus(
	session: Session,
	allWorkspaces: UserWorkspace[],
	comparison: WorkspaceComparison | undefined
): ForkStatus | undefined {
	const wsId = session.workspace_id ?? session.pending_workspace_id
	if (!wsId) return undefined
	const ws = allWorkspaces.find((w) => w.id === wsId)
	// Committed fork workspaces that disappear from the user's list
	// (deleted, archived, or access lost) are flagged unavailable so
	// the UI can render a terminal state without trying to switch into
	// them. Drafts whose pending workspace also vanished get the same
	// treatment.
	if (!ws) return session.workspace_id ? 'unavailable' : undefined
	if (!ws.parent_workspace_id) return undefined
	if (!comparison) return undefined
	const ahead = comparison.summary?.total_ahead ?? 0
	const behind = comparison.summary?.total_behind ?? 0
	if (ahead > 0 && behind > 0) return 'diverged'
	if (ahead > 0) return 'ahead'
	return 'in_sync'
}

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
	// User-archived sessions are hidden from the sidebar by default
	// (toggleable via the picker filter). Archive is reversible — distinct
	// from delete, which removes the session entirely.
	archived?: boolean
	// In-memory-only flag: the session exists but isn't written to
	// localStorage until the user sends their first message. Avoids
	// piling abandoned drafts across `+` clicks — createSession reuses
	// the existing transient if one is already open.
	transient?: boolean
}

const STORAGE_KEY = 'windmill_sessions'

// New users (empty/cleared/private-browsing localStorage) start with no
// sessions — the sidebar + /sessions page render their empty states and the
// user creates the first session with `+`. Do NOT seed placeholder sessions
// here: hardcoded example paths won't resolve for other users and render as
// "session not found".
const defaultSessions: Session[] = []

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
		// Transient (unsent) sessions stay in memory only. They get
		// materialised — and from then on written to storage — when the
		// user sends their first message.
		const toPersist = $state.snapshot(sessionState.sessions).filter((s) => !s.transient)
		localStorage.setItem(STORAGE_KEY, JSON.stringify(toPersist))
	} catch (e) {
		console.error('Failed to persist sessions', e)
	}
}

export function findSessionByName(name: string): Session | undefined {
	return sessionState.sessions.find((s) => s.name === name)
}

// Walk up parent_workspace_id to the family root, given a starting
// workspace id. Returns the input id if no parent chain is visible.
function familyRootId(id: string | undefined, all: UserWorkspace[]): string | undefined {
	if (!id) return undefined
	let cur = all.find((w) => w.id === id)
	while (cur?.parent_workspace_id) {
		const parent = all.find((w) => w.id === cur!.parent_workspace_id)
		if (!parent) break
		cur = parent
	}
	return cur?.id ?? id
}

export function createSession(): Session {
	// Reuse the existing transient session (if any) so the user can hit
	// the "+" button repeatedly without piling drafts. The transient
	// becomes a real session at first-message-send time.
	const existingTransient = sessionState.sessions.find((s) => s.transient)
	if (existingTransient) {
		sessionState.currentSessionId = existingTransient.id
		return existingTransient
	}
	const existingNumbers = sessionState.sessions
		.map((s) => /^session-(\d+)$/.exec(s.name)?.[1])
		.map((n) => (n ? parseInt(n, 10) : 0))
	const next = (existingNumbers.length ? Math.max(...existingNumbers) : 0) + 1
	// Default to the family root rather than wherever the user happens
	// to be — sessions usually start from "the canonical workspace" and
	// the picker lets them switch to a fork later.
	const currentWs = get(workspaceStore)
	const root = familyRootId(currentWs ?? undefined, get(userWorkspaces))
	const pending = root ?? currentWs
	// Friendly default summary so the header reads like "Zippy session"
	// rather than "Untitled session" — assigned at create time, the user
	// can still rename it (or it gets overwritten by an editor target).
	const adj = random_adj()
	const summary = `${adj.charAt(0).toUpperCase() + adj.slice(1)} session`
	const session: Session = {
		id: createLongHash(),
		name: `session-${next}`,
		summary,
		pending_workspace_id: pending && pending.length > 0 ? pending : undefined,
		createdAt: Date.now(),
		transient: true
	}
	sessionState.sessions = [session, ...sessionState.sessions]
	sessionState.currentSessionId = session.id
	// persistSessions() filters out transients — this call is a no-op for
	// the new draft, but kept so any other session mutations get flushed.
	persistSessions()
	return session
}

// Promote an in-memory transient session to a persisted one. No-op when
// the session isn't transient. Called by the chat manager's beforeSend
// hook so the session is only written to localStorage once the user
// commits to it by sending their first message.
export function materializeTransient(id: string): void {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s || !s.transient) return
	delete s.transient
	persistSessions()
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
		const newId = await materializeFork(fork)
		if (!newId) {
			// Real failure (not a recovered duplicate). Drop the pending
			// fork so the session falls through to the workspace-pick
			// fallback on the next call and the unavailable-banner UX
			// can take over instead of looping on the same broken intent.
			s.pending_fork = undefined
			persistSessions()
			return undefined
		}
		if (get(workspaceStore) !== newId) switchWorkspace(newId)
		s.workspace_id = newId
		s.pending_fork = undefined
		s.pending_workspace_id = undefined
		persistSessions()
		return newId
	}

	const ws = s.pending_workspace_id ?? fallback
	if (!ws) return undefined
	s.workspace_id = ws
	s.pending_workspace_id = undefined
	// `pending_workspace_id` defaults to the family root when created from
	// inside a fork, so the committed workspace can differ from the active
	// workspaceStore. Without this sync, the very first AI request's
	// `logAiChat` and tool calls read the stale fork from workspaceStore
	// while the session metadata says root. Mirrors the `switchWorkspace`
	// in the pending_fork branch above.
	if (get(workspaceStore) !== ws) syncWorkspaceTo(ws)
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

// Create a new fork workspace via the API, refresh the user-workspaces
// store, and return the new fork id. Used by both the first-send commit
// path (commitSessionWorkspace) and the move-session-to-a-new-fork path
// in the unavailable-session banner. Returns undefined on failure (a
// user-facing toast is already emitted).
//
// Self-heal: if `fork.id` is already present in the user-workspaces
// store, the previous create succeeded (whose response we apparently
// lost). Adopt it silently instead of re-POSTing — the API would
// otherwise reject with workspace_pkey. Likewise, if the API returns a
// duplicate-key error we refresh the store and adopt the existing row.
export async function materializeFork(fork: PendingFork): Promise<string | undefined> {
	if (get(userWorkspaces).some((w) => w.id === fork.id)) return fork.id
	try {
		await WorkspaceService.createWorkspaceFork({
			workspace: fork.parent_workspace_id,
			requestBody: { id: fork.id, name: fork.name }
		})
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		sendUserToast(`Created fork ${fork.name}`)
		return fork.id
	} catch (e: any) {
		const msg = String(e?.body ?? e?.message ?? e)
		if (/workspace_pkey|duplicate key/i.test(msg)) {
			// Self-heal: the create likely already succeeded. Refresh + adopt the
			// existing row. Guard this refresh — a second network failure here must
			// NOT rethrow out of materializeFork (callers rely on the
			// toast-and-return-undefined contract; an uncaught throw would bypass
			// it and propagate up through commitSessionWorkspace/beforeSend).
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
				if (get(userWorkspaces).some((w) => w.id === fork.id)) return fork.id
			} catch (refreshErr) {
				console.error('Failed to refresh workspaces during fork self-heal', refreshErr)
			}
		}
		sendUserToast(`Could not create fork: ${msg}`, true)
		return undefined
	}
}

// Re-assign a committed session to a different workspace. Used to rescue
// sessions whose original workspace was deleted / archived / had access
// revoked — the chat history (stored in IndexedDB keyed by session id) is
// preserved; only the workspace pointer changes. Drops pending fields
// since the session is already past the draft stage by definition.
export function moveSessionToWorkspace(id: string, newWorkspaceId: string) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	if (s.workspace_id === newWorkspaceId) return
	s.workspace_id = newWorkspaceId
	delete s.pending_workspace_id
	delete s.pending_fork
	persistSessions()
}

// Create a brand-new fork and re-assign a committed session to it. Used
// by the unavailable-session banner's "Create new fork" path in the
// move dropdown. On success the global workspace is switched to the
// freshly created fork.
export async function moveSessionToNewFork(
	id: string,
	fork: PendingFork
): Promise<string | undefined> {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return undefined
	const newId = await materializeFork(fork)
	if (!newId) return undefined
	if (get(workspaceStore) !== newId) switchWorkspace(newId)
	moveSessionToWorkspace(id, newId)
	return newId
}

export function setSessionArchived(id: string, archived: boolean) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	const next = archived ? true : undefined
	if (s.archived === next) return
	if (archived) s.archived = true
	else delete s.archived
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
				// Only seed pre-existing (persisted) sessions. Transient
				// sessions are freshly created via the "+" button and must
				// start with an empty chat — if the seed catches one (e.g. the
				// user clicks "New session" before this one-shot runs), it
				// would graft a previous discussion onto the new session.
				const seedable = sessionState.sessions.filter((s) => !s.transient)
				let mutated = false
				for (let i = 0; i < Math.min(seedable.length, untagged.length); i++) {
					if (!seedable[i].chatId) {
						const chatId = untagged[i].id
						const sessionId = seedable[i].id
						seedable[i].chatId = chatId
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
