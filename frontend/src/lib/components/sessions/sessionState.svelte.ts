import { BROWSER } from 'esm-env'
import { get } from 'svelte/store'
import { createLongHash } from '$lib/editorLangUtils'
import { workspaceStore } from '$lib/stores'
import type HistoryManager from '$lib/components/copilot/chat/HistoryManager.svelte'

export type SessionTarget = { kind: 'flow' | 'script' | 'app' | 'rawapp'; path: string }

export type Session = {
	id: string
	name: string
	workspace_id: string
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
		workspace_id: '',
		summary: 'testing_flow',
		target: { kind: 'flow', path: 'u/guilhempw/testing_flow' },
		createdAt: now
	},
	{
		id: createLongHash(),
		name: 'session-2',
		workspace_id: '',
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
				// Backfill workspace_id for sessions stored before this field existed.
				// We can't know the original workspace; fall back to the current one.
				const currentWorkspace = get(workspaceStore) ?? ''
				let mutated = false
				for (const s of parsed) {
					if (typeof s.workspace_id !== 'string') {
						s.workspace_id = currentWorkspace
						mutated = true
					}
				}
				if (mutated && currentWorkspace) {
					try {
						localStorage.setItem(STORAGE_KEY, JSON.stringify(parsed))
					} catch (e) {
						console.error('Failed to persist backfilled workspace_id', e)
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
	const session: Session = {
		id: createLongHash(),
		name: `session-${next}`,
		workspace_id: get(workspaceStore) ?? '',
		createdAt: Date.now()
	}
	sessionState.sessions = [session, ...sessionState.sessions]
	sessionState.currentSessionId = session.id
	persistSessions()
	return session
}

export function setSessionWorkspace(id: string, workspace_id: string) {
	const s = sessionState.sessions.find((x) => x.id === id)
	if (!s || s.workspace_id === workspace_id) return
	s.workspace_id = workspace_id
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
