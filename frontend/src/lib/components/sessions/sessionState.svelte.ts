import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { createLongHash } from '$lib/editorLangUtils'
import { random_adj } from '$lib/components/random_positive_adjetive'
import {
	enterpriseLicense,
	userStore,
	userWorkspaces,
	usersWorkspaceStore,
	workspaceStore,
	type UserWorkspace
} from '$lib/stores'
import { switchWorkspace } from '$lib/storeUtils'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
import {
	isRuleActive,
	canUserBypassRuleKind,
	protectionRulesState
} from '$lib/workspaceProtectionRules.svelte'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'
import { workspaceRootId } from './sessionScope.svelte'
import { type DBSchema, type IDBPDatabase } from 'idb'
import { userScopedDb } from '$lib/userScopedDb'
import { deleteItemsForSession } from '../copilot/chat/files/attachedFilesDB'

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
import { onUserChange, scopedKey } from '$lib/userScopedStorage'

// Kinds the in-session editor pane can host. Legacy drag-and-drop apps are
// intentionally not previewable — only code-based 'raw_app' apps are. A
// 'pipeline' target's `path` is the folder name (not a workspace item path):
// it hosts the data-pipeline graph editor for that folder, which uses its own
// fetch/draft model rather than the single-item load slots the other kinds share.
export type SessionTarget = { kind: 'flow' | 'script' | 'raw_app' | 'pipeline'; path: string }

// Useful for filtering dropdowns / pickers to "items the side panel can open".
export const EDITOR_TARGET_KINDS: ReadonlySet<SessionTarget['kind']> = new Set([
	'flow',
	'script',
	'raw_app',
	'pipeline'
])

// Whether the session points at a workspace that is itself a fork (i.e.
// has a parent). Used by the sidebar to pick between a root (Building)
// icon and a fork icon.
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
	// Stable root workspace id used only for sidebar grouping. Lifecycle follows
	// workspace_id, not this field. Root sessions store the same id in both fields.
	workspace_root_id?: string
	chatId?: string
	summary?: string
	summarySource?: SessionSummarySource
	createdAt: number
	// User-archived sessions are hidden from the sidebar by default
	// (toggleable via the picker filter). Archive is reversible — distinct
	// from delete, which removes the session entirely.
	archived?: boolean
	// Set when `archived` was applied because the session's workspace was
	// archived (not by the user). Lets reconciliation auto-unarchive the session
	// when the workspace is unarchived, while leaving user-archived sessions be.
	archivedByWorkspace?: boolean
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
	// Preview tabs the user opened in the sessions page, restored when the
	// session is reopened.
	previewTabs?: SessionPreviewTab[]
	// Which preview tab was active. Falls back to the first tab on restore.
	activePreviewTabId?: string
	// Whether the user collapsed the preview panel for this session (to give the
	// chat full width). Per-session so each session restores its own layout.
	previewCollapsed?: boolean
}

// One preview tab: `url` is the URL we command the iframe to load, `loc` the
// last observed location (see the sessions page for the url/loc split).
export type SessionPreviewTab = { id: string; url: string; loc: string }

// Sessions live in one per-user IndexedDB, one record per session in the
// `sessions` store keyed by `id`. IndexedDB is the sole store — no localStorage
// fallback. The bare localStorage keys below are the oldest (pre-namespacing)
// source, claimed once during the legacy migration.
const SESSIONS_DB = 'windmill-sessions'
const LEGACY_SESSIONS_KEY = 'windmill_sessions'
const LEGACY_LAST_SEEN_KEY = 'windmill_sessions_last_seen_counts'
// The single unsent (transient) draft, kept in localStorage (user-scoped) so a
// reload doesn't lose what the user set up before their first message: name,
// workspace/fork choice, editor target, preview tabs and the typed-but-unsent
// prompt.
const TRANSIENT_DRAFT_KEY = 'wm_session_transient_draft'

interface SessionSchema extends DBSchema {
	sessions: { key: string; value: Session }
}

// Normalise legacy localStorage records in place: drop empty-string
// workspace_id (older drafts used '' as a missing marker), drop the retired
// `target` field (the preview is tab-driven now), and coerce unknown
// summarySource values. Operates on raw parsed JSON, so the record is loosely typed.
function normalizeLegacySession(s: Record<string, any>): void {
	if (s.workspace_id === '') delete s.workspace_id
	delete s.target
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

function sessionRootId(s: Session): string | undefined {
	return (
		s.workspace_root_id ??
		workspaceRootId(s.workspace_id ?? s.pending_workspace_id, get(userWorkspaces)) ??
		s.workspace_id ??
		s.pending_workspace_id
	)
}

// Whether a session belongs to the active workspace's family. Only in-memory
// drafts (transient) may lack a root and still count as in-family — they follow
// the user until a workspace is picked. A persisted session with no workspace
// binding must fail closed: counting it as in-family would surface it in every
// family, including freshly created workspaces.
export function sessionInCurrentFamily(s: Session): boolean {
	const currentRoot = workspaceRootId(get(workspaceStore) ?? undefined, get(userWorkspaces))
	if (!currentRoot) return true
	const root = sessionRootId(s)
	if (root === undefined) return !!s.transient
	return root === currentRoot
}

function ensureSessionRootId(s: Session): boolean {
	if (s.workspace_root_id || s.transient) return false
	const workspaceId = s.workspace_id ?? s.pending_workspace_id
	if (workspaceId && get(userWorkspaces).length === 0) return false
	const root = sessionRootId(s)
	if (!root) return false
	s.workspace_root_id = root
	return true
}

// Recompute workspace_root_id from the live parent chain, replacing a stale
// stored value. A fork family is re-rooted when an ancestor is deleted: the
// FK's ON DELETE SET NULL nulls the child's parent_workspace_id, so the topmost
// member shifts down. Without this, a sub-fork session keeps a root pointing at
// the deleted ancestor and drops out of the sidebar (grouped under a dead root)
// even though its own workspace is still alive. Only re-roots when the
// workspace resolves in the live list — otherwise workspaceRootId falls back to
// the id itself and would clobber a valid root for a merely-unavailable workspace.
function refreshSessionRootId(s: Session): boolean {
	if (s.transient) return false
	const workspaceId = s.workspace_id ?? s.pending_workspace_id
	if (!workspaceId) return false
	const all = get(userWorkspaces)
	if (!all.some((w) => w.id === workspaceId)) return false
	const root = workspaceRootId(workspaceId, all)
	if (!root || root === s.workspace_root_id) return false
	s.workspace_root_id = root
	return true
}

const sessionsDb = userScopedDb<SessionSchema>(SESSIONS_DB, {
	version: 1,
	upgrade(db) {
		if (!db.objectStoreNames.contains('sessions')) {
			db.createObjectStore('sessions', { keyPath: 'id' })
		}
	},
	async migrate(db) {
		await migrateSessionsFromLocalStorage(db)
	}
})

// Starts empty: the list is hydrated from the user's IndexedDB by the
// onUserChange handler below once the logged-in email resolves (async, after
// layout load). Opening eagerly here would touch a DB before we know who is
// logged in.
export const sessionState = $state<{
	sessions: Session[]
	currentSessionId: string | undefined
	// False until the first IndexedDB hydration completes. Consumers gate their
	// "not found" / empty states on it — before that, an empty list only means
	// "still loading", not "the user has no sessions".
	hydrated: boolean
}>({
	sessions: [],
	currentSessionId: undefined,
	hydrated: false
})

type TransientDraft = Session & {
	prompt?: string
}

// The unsent prompt for the current transient session, held here so every
// draft write (which snapshots only the Session record) can carry it along.
let transientPrompt: { sessionId: string; text: string } | undefined

function writeTransientDraft(s: Session): void {
	const key = scopedKey(TRANSIENT_DRAFT_KEY)
	if (!key) return
	const draft: TransientDraft = {
		...($state.snapshot(s) as Session),
		prompt: transientPrompt?.sessionId === s.id ? transientPrompt.text : undefined
	}
	storeLocalSetting(key, JSON.stringify(draft))
}

function readTransientDraft(): TransientDraft | undefined {
	const key = scopedKey(TRANSIENT_DRAFT_KEY)
	if (!key) return undefined
	const raw = getLocalSetting(key)
	if (!raw) return undefined
	try {
		const d = JSON.parse(raw)
		if (!d || typeof d.id !== 'string' || typeof d.name !== 'string') return undefined
		return d as TransientDraft
	} catch {
		return undefined
	}
}

function clearTransientDraft(): void {
	const key = scopedKey(TRANSIENT_DRAFT_KEY)
	if (key) storeLocalSetting(key, undefined)
	transientPrompt = undefined
}

// Debounced write-behind of the chat input for a transient session, so the
// typed-but-unsent prompt survives a reload with the rest of the draft.
let transientPromptFlushHandle: ReturnType<typeof setTimeout> | undefined
export function queueTransientDraftPrompt(sessionId: string, text: string): void {
	const s = sessionState.sessions.find((x) => x.id === sessionId)
	if (!s?.transient) return
	transientPrompt = { sessionId, text }
	clearTimeout(transientPromptFlushHandle)
	transientPromptFlushHandle = setTimeout(() => writeTransientDraft(s), 400)
}

// Read back the restored draft prompt when the session's runtime (and its chat
// manager) is created. Peek, not take: later draft writes keep carrying it.
export function peekTransientDraftPrompt(sessionId: string): string | undefined {
	return transientPrompt?.sessionId === sessionId ? transientPrompt.text : undefined
}

// Write-behind a single session record. Transient (unsent) sessions are not
// written to IndexedDB — they live in memory plus a single localStorage draft
// slot until materializeTransient() promotes them at first send.
// Awaits DB-open so a write racing hydration still lands; no-ops (degrades to
// in-memory) when the DB can't be opened. In-memory $state is the read surface,
// so callers fire-and-forget.
export async function putSession(s: Session): Promise<void> {
	if (!BROWSER) return
	if (s.transient) {
		writeTransientDraft(s)
		return
	}
	// Never resurrect a session whose committed workspace is gone. A live runtime
	// can still write through here after reconciliation deletes its record (chatId
	// seed, unread watermark), so guard once the workspace list is loaded.
	if (s.workspace_id) {
		const all = get(userWorkspaces)
		if (all.length > 0 && !all.some((w) => w.id === s.workspace_id)) return
	}
	ensureSessionRootId(s)
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
async function hydrateSessions({ dropTransients = false } = {}): Promise<void> {
	// Transient (unsent) drafts live only in memory and belong to the current
	// user; preserve them across an intra-user reconcile, but drop them on a user
	// change so one user's draft (and its pending fork/workspace state) never
	// bleeds into the next user's list.
	const transients = dropTransients ? [] : sessionState.sessions.filter((s) => s.transient)
	const db = await sessionsDb.whenReady()
	if (!db) {
		sessionState.sessions = transients
		return
	}
	try {
		const all = await db.getAll('sessions')
		const changed = all.filter((s) => ensureSessionRootId(s))
		for (const s of changed) await db.put('sessions', s)
		all.sort((a, b) => b.createdAt - a.createdAt)
		// Restore the (user-scoped) unsent draft, unless it already materialised
		// (present in the DB — e.g. sent from another browser tab) or the same
		// draft is still live in memory.
		const draft = readTransientDraft()
		if (draft) {
			if (all.some((s) => s.id === draft.id)) {
				clearTransientDraft()
			} else if (!transients.some((s) => s.id === draft.id)) {
				const { prompt, ...rec } = draft
				transients.push({ ...rec, transient: true })
				if (prompt) transientPrompt = { sessionId: rec.id, text: prompt }
			}
		}
		sessionState.sessions = [...transients, ...all]
	} catch (e) {
		console.error('Failed to load sessions from IndexedDB', e)
		sessionState.sessions = transients
	}
}

export type WorkspaceLifecycleStatus = 'active' | 'archived' | 'deleted'

// The never-orphaned rule as a pure function of (session, its workspace's
// status) — no IO, so the whole truth table is unit-testable. An `undefined`
// status (workspace absent from the queried set) is a no-op, never a delete.
//   deleted  → delete the session
//   archived → archive it, tagged archivedByWorkspace (idempotent)
//   active   → auto-unarchive iff WE archived it (has archivedByWorkspace)
export function decideSessionLifecycle(
	session: Session,
	status: WorkspaceLifecycleStatus | undefined
): { action: 'delete' | 'archive' | 'unarchive' | 'noop'; patch?: Partial<Session> } {
	if (status === 'deleted') return { action: 'delete' }
	if (status === 'archived') {
		return session.archived
			? { action: 'noop' }
			: { action: 'archive', patch: { archived: true, archivedByWorkspace: true } }
	}
	if (status === 'active' && session.archivedByWorkspace) {
		return { action: 'unarchive', patch: { archived: undefined, archivedByWorkspace: undefined } }
	}
	return { action: 'noop' }
}

// Apply a decision patch in place: `undefined` removes the key (the unarchive
// path needs the flags gone, not set to undefined), any other value assigns.
function applyLifecyclePatch(session: Session, patch: Partial<Session>): void {
	for (const [k, v] of Object.entries(patch)) {
		if (v === undefined) delete (session as Record<string, unknown>)[k]
		else (session as Record<string, unknown>)[k] = v
	}
}

// Workspace switches reconcile too (to catch a workspace deleted/archived on
// another device), but throttled so rapid switching doesn't spam the status
// endpoint. Mutation-driven reconciles are unthrottled.
let lastReconcileAt = 0
const RECONCILE_THROTTLE_MS = 30_000

// Reconcile every stored session against its workspace's lifecycle. Sessions are
// client-only, so the backend can't delete/archive them directly — instead the
// client asks the backend for the status of every workspace its sessions
// reference and applies the rule (see decideSessionLifecycle) the user can't see
// happen otherwise. Reached via reconcileAfterWorkspaceChange (mutations),
// load, and a throttled workspace switch.
export async function reconcileSessionsLifecycle(): Promise<void> {
	if (!BROWSER) return
	lastReconcileAt = Date.now()
	const db = await sessionsDb.whenReady()
	if (!db) return
	const wsIds = new Set<string>()
	const sessions = await db.getAll('sessions')
	for (const s of sessions) if (s.workspace_id) wsIds.add(s.workspace_id)
	if (wsIds.size === 0) return

	let status: Record<string, 'active' | 'archived' | 'deleted'>
	try {
		status = await WorkspaceService.getSessionWorkspaceStatus({
			requestBody: { workspace_ids: [...wsIds] }
		})
	} catch (e) {
		console.error('Failed to reconcile session lifecycle', e)
		return
	}

	const deletedIds = new Set<string>()
	try {
		for (const s of sessions) {
			if (!s.workspace_id) continue
			const { action, patch } = decideSessionLifecycle(s, status[s.workspace_id])
			if (action === 'delete') {
				await db.delete('sessions', s.id)
				// GC linked files too, matching deleteSession — a record-only delete
				// here would orphan the session's attached-file blobs/handles.
				void deleteItemsForSession(s.id)
				deletedIds.add(s.id)
				continue
			}
			let changed = action === 'archive' || action === 'unarchive'
			if (changed && patch) applyLifecyclePatch(s, patch)
			// Re-root surviving sessions whose family topmost member shifted (an
			// ancestor was deleted); fall back to backfilling a missing root.
			if (refreshSessionRootId(s) || ensureSessionRootId(s)) changed = true
			if (changed) await db.put('sessions', s)
		}
	} catch (e) {
		// The connection can go stale mid-loop (user switch reopens the per-user
		// DB); this pass is best-effort — the next reconcile trigger retries.
		console.error('Failed to reconcile session lifecycle', e)
		return
	}
	await hydrateSessions()
	// If the session the user was on lived in a now-deleted workspace, it was just
	// removed — drop the dangling pointer so the page falls back to "no session
	// selected" instead of a ghost.
	if (sessionState.currentSessionId && deletedIds.has(sessionState.currentSessionId)) {
		sessionState.currentSessionId = undefined
	}
}

// The single seam for "a workspace just changed — bring sessions back in sync."
// Refresh the workspace list FIRST — both reconcile and the putSession guard
// read it, so it must reflect the change before reconcile runs — then reconcile.
// Call this from every workspace create/delete/archive/unarchive site.
export async function reconcileAfterWorkspaceChange(): Promise<void> {
	if (!BROWSER) return
	try {
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
	} catch (e) {
		console.error('Failed to refresh workspace list before reconcile', e)
	}
	await reconcileSessionsLifecycle()
}

// Count non-transient sessions committed to a given workspace — used to warn the
// user, before archiving/deleting a workspace, how many AI sessions go with it.
export async function countSessionsForWorkspace(workspaceId: string): Promise<number> {
	if (!BROWSER) return 0
	const db = await sessionsDb.whenReady()
	if (!db) return 0
	try {
		const all = await db.getAll('sessions')
		return all.filter((s) => s.workspace_id === workspaceId && !s.transient).length
	} catch {
		return 0
	}
}

export async function archiveSessionsForWorkspace(workspaceId: string): Promise<void> {
	if (!BROWSER || !workspaceId) return
	const db = await sessionsDb.whenReady()
	if (!db) return
	const all = await db.getAll('sessions')
	for (const s of all) {
		// Skip sessions the user already archived: tagging them archivedByWorkspace
		// would make a later workspace-unarchive auto-restore them, discarding the
		// user's manual archive. Mirrors decideSessionLifecycle's already-archived noop.
		if (s.transient || s.workspace_id !== workspaceId || s.archived) continue
		s.archived = true
		s.archivedByWorkspace = true
		ensureSessionRootId(s)
		await db.put('sessions', s)
	}
	for (const s of sessionState.sessions) {
		if (s.transient || s.workspace_id !== workspaceId || s.archived) continue
		s.archived = true
		s.archivedByWorkspace = true
	}
}

export async function deleteSessionsForWorkspace(workspaceId: string): Promise<void> {
	if (!BROWSER || !workspaceId) return
	const db = await sessionsDb.whenReady()
	if (!db) return
	const all = await db.getAll('sessions')
	const ids = new Set(
		all.filter((s) => s.workspace_id === workspaceId && !s.transient).map((s) => s.id)
	)
	for (const id of ids) {
		await db.delete('sessions', id)
		// GC linked files too (matches deleteSession) so a workspace teardown
		// doesn't leave the sessions' attached-file blobs/handles orphaned.
		void deleteItemsForSession(id)
	}
	sessionState.sessions = sessionState.sessions.filter((s) => !ids.has(s.id))
	if (sessionState.currentSessionId && ids.has(sessionState.currentSessionId)) {
		sessionState.currentSessionId = undefined
	}
}

// Re-hydrate on user (email) change. The new user's persisted sessions are
// re-read from their own scoped DB (different email → different DB name); the
// in-memory list is rebuilt from scratch — including dropping the previous
// user's transient drafts — and the active-session pointer is cleared, so one
// user's sessions never bleed into another.
onUserChange(async (email, prevEmail) => {
	if (!BROWSER) return
	await hydrateSessions({ dropTransients: prevEmail !== email })
	// onUserChange also fires at registration time, before the email resolves —
	// that hydration is an empty no-op and must not clear the loading state.
	if (email !== undefined) sessionState.hydrated = true
	if (prevEmail !== undefined && prevEmail !== email) {
		sessionState.currentSessionId = undefined
	}
	// Load-time reconcile: catch workspaces archived/deleted while away.
	void reconcileSessionsLifecycle()
})

// Workspace switches do not reload or clear the active session. The sidebar
// filters the single loaded session list by workspace_root_id, while the current
// chat survives workspace switches.
if (BROWSER) {
	let lastWorkspace: string | undefined
	let initialized = false
	workspaceStore.subscribe((ws) => {
		const isSwitch = initialized && ws !== lastWorkspace
		initialized = true
		lastWorkspace = ws
		if (isSwitch && Date.now() - lastReconcileAt > RECONCILE_THROTTLE_MS) {
			void reconcileSessionsLifecycle()
		}
	})
}

export function findSessionByName(name: string): Session | undefined {
	return sessionState.sessions.find((s) => s.name === name)
}

export function createSession(): Session {
	// Reuse the existing transient session (if any) so the user can hit
	// the "+" button repeatedly without piling drafts. The transient
	// becomes a real session at first-message-send time. Only a transient
	// from the active workspace family qualifies — reusing one left over
	// from another family would hand the user a session still acting on
	// that family. A cross-family leftover is dropped instead (it was
	// never sent, so only the draft slot holds it).
	const existingTransient = sessionState.sessions.find((s) => s.transient)
	if (existingTransient) {
		if (sessionInCurrentFamily(existingTransient)) {
			sessionState.currentSessionId = existingTransient.id
			return existingTransient
		}
		sessionState.sessions = sessionState.sessions.filter((s) => s.id !== existingTransient.id)
		clearTransientDraft()
	}
	const existingNumbers = sessionState.sessions
		.map((s) => /^session-(\d+)$/.exec(s.name)?.[1])
		.map((n) => (n ? parseInt(n, 10) : 0))
	const next = (existingNumbers.length ? Math.max(...existingNumbers) : 0) + 1
	// Start in the workspace you're in. The one exception: a root you can't
	// deploy to (locked, no bypass) steers to its dev, since a session there
	// couldn't edit anything. The picker lets you switch.
	const currentWs = get(workspaceStore)
	const devOfCurrent = currentWs
		? findCanonicalDevWorkspace(currentWs, get(userWorkspaces))?.id
		: undefined
	// Only trust the deploy check once the active workspace's rules have actually loaded: until then
	// `isRuleActive` reads an empty ruleset and fails open, which would default a new session onto a
	// locked prod. Treat "not yet loaded for currentWs" as not-deployable so we steer to the dev (always
	// editable) when one exists; the picker still lets the user switch back once rules resolve.
	const rulesLoadedForCurrent =
		protectionRulesState.rulesets !== undefined && protectionRulesState.workspace === currentWs
	const canDeployHere =
		rulesLoadedForCurrent &&
		(!isRuleActive('DisableDirectDeployment') ||
			canUserBypassRuleKind('DisableDirectDeployment', get(userStore)))
	const pending = devOfCurrent && !canDeployHere ? devOfCurrent : currentWs
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
	// Transient until first send: no DB record yet, but the draft slot keeps it
	// (name, workspace/fork choice, prompt) across reloads.
	writeTransientDraft(session)
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
	// Promoted to IndexedDB — the localStorage draft slot is now stale.
	clearTransientDraft()
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
		s.workspace_id = newId
		s.pending_fork = undefined
		s.pending_workspace_id = undefined
		s.workspace_root_id = workspaceRootId(newId, get(userWorkspaces)) ?? newId
		await putSession(s)
		// The global workspaceStore is intentionally left untouched: the session
		// chat targets its own workspace via AIChatManager.operatingWorkspace, so
		// committing must not yank the user's active (navigation-mode) workspace.
		return newId
	}

	const ws = s.pending_workspace_id ?? fallback
	if (!ws) return undefined
	s.workspace_id = ws
	s.pending_workspace_id = undefined
	s.workspace_root_id = workspaceRootId(ws, get(userWorkspaces)) ?? ws
	await putSession(s)
	// The global workspaceStore is intentionally left untouched (see the fork
	// branch above): the session chat reads its committed workspace through the
	// manager's workspace resolver, not the active workspaceStore.
	return ws
}

// Effective workspace for scope/routing — committed if set, otherwise the
// pre-send pending pick (which defaults to the workspace at create time).
// Pending forks route via their parent until creation lands.
export function getEffectiveWorkspaceId(session: Session): string | undefined {
	return session.workspace_id ?? session.pending_workspace_id
}

// Persist the session's preview tabs. Fire-and-forget write-behind (transient
// sessions land in the localStorage draft slot).
export function setSessionTabs(id: string, tabs: SessionPreviewTab[], activeTabId: string): void {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	s.previewTabs = tabs.map((t) => ({ ...t }))
	s.activePreviewTabId = activeTabId
	void putSession(s)
}

// Persist whether the preview panel is collapsed for this session. Fire-and-forget
// write-behind (transient sessions land in the localStorage draft slot).
export function setSessionPreviewCollapsed(id: string, collapsed: boolean): void {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s || !!s.previewCollapsed === collapsed) return
	s.previewCollapsed = collapsed
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
export async function moveSessionToWorkspace(id: string, newWorkspaceId: string): Promise<void> {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	if (s.workspace_id === newWorkspaceId) return
	s.workspace_id = newWorkspaceId
	delete s.pending_workspace_id
	delete s.pending_fork
	s.workspace_root_id = workspaceRootId(newWorkspaceId, get(userWorkspaces)) ?? newWorkspaceId
	await putSession(s)
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
	// Persist the session before switching so the target workspace can show it
	// immediately in the root-filtered sidebar.
	await moveSessionToWorkspace(id, newId)
	if (get(workspaceStore) !== newId) switchWorkspace(newId)
	return newId
}

export function setSessionArchived(id: string, archived: boolean) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	const next = archived ? true : undefined
	if (s.archived === next && (archived || !s.archivedByWorkspace)) return
	if (archived) s.archived = true
	else {
		delete s.archived
		delete s.archivedByWorkspace
	}
	void putSession(s)
}

export function deleteSession(id: string) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s) return
	if (s.transient) clearTransientDraft()
	sessionState.sessions = sessionState.sessions.filter((x) => x.id !== id)
	if (sessionState.currentSessionId === id) {
		sessionState.currentSessionId = sessionState.sessions[0]?.id
	}
	void deleteSessionRecord(id)
	// GC any linked files persisted for this session.
	void deleteItemsForSession(id)
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
