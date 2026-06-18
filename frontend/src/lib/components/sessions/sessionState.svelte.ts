import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { createLongHash } from '$lib/editorLangUtils'
import { random_adj } from '$lib/components/random_positive_adjetive'
import {
	enterpriseLicense,
	userWorkspaces,
	usersWorkspaceStore,
	workspaceStore,
	type UserWorkspace
} from '$lib/stores'
import { switchWorkspace } from '$lib/storeUtils'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { userScopedDb } from '$lib/userScopedDb'
import type { DBSchema, IDBPDatabase } from 'idb'

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
import { onUserChange } from '$lib/userScopedStorage'

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

export type SessionSummarySource = 'placeholder' | 'generated' | 'manual'

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
	summarySource?: SessionSummarySource
	createdAt: number
	// User-archived sessions are hidden from the sidebar by default
	// (toggleable via the picker filter). Archive is reversible — distinct
	// from delete, which removes the session entirely.
	archived?: boolean
	// In-memory-only flag: the session exists but isn't written to
	// IndexedDB until the user sends their first message. Avoids
	// piling abandoned drafts across `+` clicks — createSession reuses
	// the existing transient if one is already open.
	transient?: boolean
	// Per-session unread watermark: the displayMessages count the last time
	// the user was on this session's page. Compared against the runtime's
	// current message count to derive the unread badge (see sessionUnread).
	// Travels on the record so it is scoped, migrated, and deleted with it.
	lastSeenCount?: number
}

// Sessions live in a per-user IndexedDB (windmill-sessions::email), one record
// per session in the `sessions` store keyed by `id`. IndexedDB is the sole
// store — no localStorage fallback. The bare localStorage keys below are the
// pre-namespacing source migrated once into the first connecting user's DB.
const SESSIONS_DB = 'windmill-sessions'
const LEGACY_SESSIONS_KEY = 'windmill_sessions'
const LEGACY_LAST_SEEN_KEY = 'windmill_sessions_last_seen_counts'

interface SessionSchema extends DBSchema {
	sessions: { key: string; value: Session }
}

// Normalise legacy localStorage records in place: drop empty-string
// workspace_id (older drafts used '' as a missing marker), migrate the
// deprecated 'rawapp' target.kind, and coerce unknown summarySource values.
// Operates on raw parsed JSON, so the record is loosely typed.
function normalizeLegacySession(s: Record<string, any>): void {
	if (s.workspace_id === '') delete s.workspace_id
	if (s.target?.kind === 'rawapp') s.target.kind = 'raw_app'
	if (
		s.summarySource !== undefined &&
		s.summarySource !== 'placeholder' &&
		s.summarySource !== 'generated' &&
		s.summarySource !== 'manual'
	) {
		s.summarySource = 'manual'
	}
}

// One-shot claim of the pre-namespacing localStorage data into the per-user DB.
// Runs once per scoped DB (via userScopedDb's migrate gate) and only when the
// DB is still empty — the first user to connect on a previously single-user
// browser inherits the sessions; later users start clean. The bare watermark
// map is folded onto each record as lastSeenCount. Both bare keys are deleted
// so nothing is left to re-claim. Intermediate `::email` keys (this branch's
// throwaway dev data) are intentionally not migrated.
async function migrateSessionsFromLocalStorage(db: IDBPDatabase<SessionSchema>): Promise<void> {
	// Only claim into a still-empty DB, but ALWAYS fall through to delete the bare
	// keys below — even when the DB is already populated. Otherwise a partially
	// failed prior migration (puts committed, deletes didn't) would leave the bare
	// keys for a different empty-DB user to later claim: the exact leak this closes.
	const alreadyPopulated = (await db.count('sessions')) > 0
	const raw = getLocalSetting(LEGACY_SESSIONS_KEY)
	if (!alreadyPopulated && raw) {
		try {
			const parsed = JSON.parse(raw)
			if (Array.isArray(parsed) && parsed.length > 0) {
				let watermarks: Record<string, number> = {}
				const wmRaw = getLocalSetting(LEGACY_LAST_SEEN_KEY)
				if (wmRaw) {
					try {
						watermarks = JSON.parse(wmRaw)
					} catch {
						/* ignore malformed watermark map */
					}
				}
				const tx = db.transaction('sessions', 'readwrite')
				for (const s of parsed) {
					if (!s || typeof s.id !== 'string' || s.transient) continue
					normalizeLegacySession(s)
					const count = watermarks[s.id]
					if (typeof count === 'number') s.lastSeenCount = count
					tx.store.put(s as Session)
				}
				await tx.done
			}
		} catch (e) {
			console.error('Failed to migrate legacy sessions into IndexedDB', e)
		}
	}
	// Drop the bare keys unconditionally (claimed, empty, malformed, or stale
	// leftovers from a partially-failed prior migration) so a later different user
	// on this browser never re-inherits them.
	storeLocalSetting(LEGACY_SESSIONS_KEY, undefined)
	storeLocalSetting(LEGACY_LAST_SEEN_KEY, undefined)
}

const sessionsDb = userScopedDb<SessionSchema>(SESSIONS_DB, {
	version: 1,
	upgrade(db) {
		if (!db.objectStoreNames.contains('sessions')) {
			db.createObjectStore('sessions', { keyPath: 'id' })
		}
	},
	migrate: migrateSessionsFromLocalStorage
})

// Starts empty: the list is hydrated from the user's IndexedDB by the
// onUserChange handler below once the logged-in email resolves (async, after
// layout load). Opening eagerly here would touch a DB before we know who is
// logged in.
export const sessionState = $state<{
	sessions: Session[]
	currentSessionId: string | undefined
}>({
	sessions: [],
	currentSessionId: undefined
})

// Write-behind a single session record. Transient (unsent) sessions stay in
// memory only — they materialise via materializeTransient() at first send.
// Awaits DB-open so a write racing hydration still lands; no-ops (degrades to
// in-memory) when the DB can't be opened. In-memory $state is the read surface,
// so callers fire-and-forget.
export async function putSession(s: Session): Promise<void> {
	if (!BROWSER || s.transient) return
	const db = await sessionsDb.whenReady()
	if (!db) return
	try {
		await db.put('sessions', $state.snapshot(s))
	} catch (e) {
		console.error('Failed to persist session', e)
	}
}

export async function deleteSessionRecord(id: string): Promise<void> {
	if (!BROWSER) return
	const db = await sessionsDb.whenReady()
	if (!db) return
	try {
		await db.delete('sessions', id)
	} catch (e) {
		console.error('Failed to delete session record', e)
	}
}

// Load the in-memory list from the user's DB. getAll() returns records in key
// (id) order; sort by createdAt descending to reproduce the newest-first order
// createSession() maintains (it prepends). whenReady() reopens for the current
// user automatically, so this also handles user switch; an absent DB (logged
// out / open failed) yields an empty list.
async function hydrateSessions(): Promise<void> {
	const db = await sessionsDb.whenReady()
	if (!db) {
		sessionState.sessions = []
		return
	}
	try {
		const all = await db.getAll('sessions')
		all.sort((a, b) => b.createdAt - a.createdAt)
		sessionState.sessions = all
	} catch (e) {
		console.error('Failed to load sessions from IndexedDB', e)
		sessionState.sessions = []
	}
}

// Re-hydrate whenever the logged-in email resolves or changes. On logout
// (email → undefined) or a genuine user switch (X → Y) the in-memory list and
// active-session pointer reset so one user's sessions never bleed into another.
onUserChange(async (email, prevEmail) => {
	if (!BROWSER) return
	await hydrateSessions()
	if (prevEmail !== undefined && prevEmail !== email) {
		sessionState.currentSessionId = undefined
	}
})

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
		summarySource: 'placeholder',
		pending_workspace_id: pending && pending.length > 0 ? pending : undefined,
		createdAt: Date.now(),
		transient: true
	}
	sessionState.sessions = [session, ...sessionState.sessions]
	sessionState.currentSessionId = session.id
	// The new session is transient (in-memory only) until first send, so there
	// is nothing to write to the DB yet.
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
	void putSession(s)
}

export function setSessionPendingWorkspace(id: string, workspace_id: string) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	const changed = s.pending_workspace_id !== workspace_id || s.pending_fork !== undefined
	s.pending_workspace_id = workspace_id
	// Picking an existing workspace cancels any pending fork intent.
	s.pending_fork = undefined
	if (changed) void putSession(s)
}

// Records the user's intent to create a new fork without firing the API
// call yet. Routing/scope stay on the parent workspace until commit.
export function setSessionPendingFork(id: string, fork: PendingFork) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.pending_fork = { ...fork }
	s.pending_workspace_id = fork.parent_workspace_id
	void putSession(s)
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
		// Defense-in-depth against a stale pending_fork (e.g. staged before
		// another workspace was created, or any future entry point): a fork is a
		// new workspace, and community edition caps the number of non-'admins'
		// workspaces (backend _check_nb_of_workspaces). An enterprise license
		// lifts the cap. Block the commit with an explicit error rather than
		// letting materializeFork hit a backend rejection. Keep the pending fork
		// set so the block persists until the user picks a non-fork workspace
		// (setSessionPendingWorkspace clears it).
		const CE_MAX_NON_ADMIN_WORKSPACES = 2
		const nonAdminWorkspaceCount = get(userWorkspaces).filter((w) => w.id !== 'admins').length
		if (!get(enterpriseLicense) && nonAdminWorkspaceCount >= CE_MAX_NON_ADMIN_WORKSPACES) {
			throw new Error(
				`Community edition is limited to ${CE_MAX_NON_ADMIN_WORKSPACES + 1} workspaces — archive a workspace or pick one to run in`
			)
		}
		const newId = await materializeFork(fork)
		if (!newId) {
			// Real failure (not a recovered duplicate). Drop the pending
			// fork so the session falls through to the workspace-pick
			// fallback on the next call and the unavailable-banner UX
			// can take over instead of looping on the same broken intent.
			s.pending_fork = undefined
			void putSession(s)
			return undefined
		}
		if (get(workspaceStore) !== newId) switchWorkspace(newId)
		s.workspace_id = newId
		s.pending_fork = undefined
		s.pending_workspace_id = undefined
		void putSession(s)
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
	void putSession(s)
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
	if (!s.summary && summary) {
		s.summary = summary
		s.summarySource = 'generated'
	}
	void putSession(s)
}

export function selectSession(id: string) {
	sessionState.currentSessionId = id
}

export function renameSession(id: string, newSummary: string) {
	const trimmed = newSummary.trim()
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.summary = trimmed.length > 0 ? trimmed : undefined
	s.summarySource = 'manual'
	void putSession(s)
}

export function setGeneratedSessionSummary(
	id: string,
	newSummary: string,
	expectedChatId?: string
): boolean {
	const trimmed = newSummary.trim()
	if (!trimmed) return false
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return false
	if (expectedChatId && s.chatId !== expectedChatId) return false
	if (s.summarySource !== 'placeholder') return false
	s.summary = trimmed
	s.summarySource = 'generated'
	void putSession(s)
	return true
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
	void putSession(s)
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
	void putSession(s)
}

export function deleteSession(id: string) {
	const idx = sessionState.sessions.findIndex((s) => s.id === id)
	if (idx < 0) return
	sessionState.sessions = sessionState.sessions.filter((s) => s.id !== id)
	if (sessionState.currentSessionId === id) {
		sessionState.currentSessionId = sessionState.sessions[0]?.id
	}
	void deleteSessionRecord(id)
}

export function setSessionChatId(sessionId: string, chatId: string) {
	const s = sessionState.sessions.find((x) => x.id === sessionId)
	if (s && s.chatId !== chatId) {
		s.chatId = chatId
		void putSession(s)
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
				for (let i = 0; i < Math.min(seedable.length, untagged.length); i++) {
					if (!seedable[i].chatId) {
						const chatId = untagged[i].id
						const sessionId = seedable[i].id
						seedable[i].chatId = chatId
						await historyManager.tagChatWithSession(chatId, sessionId)
						void putSession(seedable[i])
					}
				}
			} catch (e) {
				console.error('Failed to seed chat ids from history', e)
			}
		})()
	}
	return seedPromise
}
