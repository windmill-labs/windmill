import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import { enterSessionMode } from './sessionSwitch.svelte'
import { sessionState, type Session } from './sessionState.svelte'
import { registerEditorSessionSource } from './editorSessionSource'
import { usersWorkspaceStore, workspaceStore, type UserWorkspace } from '$lib/stores'

vi.mock('$lib/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }))
import { goto } from '$lib/navigation'
// `openEditorInSession` lazy-imports the runtime (monaco-heavy) — stub it so the
// seed path is testable in node.
vi.mock('./sessionRuntime.svelte', () => ({ resetSessionPreviewTabs: vi.fn() }))

function session(over: Partial<Session> = {}): Session {
	return { id: 's1', name: 'sess', createdAt: 0, ...over }
}
function ws(id: string, parent?: string): UserWorkspace {
	return { id, name: id, parent_workspace_id: parent } as unknown as UserWorkspace
}

// Two-family fixture: rootA (with forkA) and rootB. Returns a restore fn.
function withTwoFamilies(activeWorkspace: string): () => void {
	const prevUsers = get(usersWorkspaceStore)
	const prevWs = get(workspaceStore)
	usersWorkspaceStore.set({
		email: 't@t',
		workspaces: [ws('rootA'), ws('forkA', 'rootA'), ws('rootB')]
	} as never)
	workspaceStore.set(activeWorkspace)
	return () => {
		usersWorkspaceStore.set(prevUsers)
		workspaceStore.set(prevWs)
	}
}

describe('enterSessionMode — restore is scoped to the active family', () => {
	beforeEach(() => {
		vi.mocked(goto).mockClear()
	})

	it('keeps the current session when it belongs to the active family', async () => {
		const restore = withTwoFamilies('forkA')
		const prevCurrent = sessionState.currentSessionId
		const inFamily = session({ id: 'sw-in-family', name: 'session-911', workspace_id: 'rootA' })
		sessionState.sessions.push(inFamily)
		sessionState.currentSessionId = 'sw-in-family'
		try {
			await enterSessionMode()
			expect(sessionState.currentSessionId).toBe('sw-in-family')
			expect(goto).toHaveBeenCalledWith('/sessions?session_name=session-911', {
				replaceState: false
			})
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== 'sw-in-family')
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})

	it('skips a cross-family current session in favor of the most recent in-family one', async () => {
		const restore = withTwoFamilies('rootB')
		const prevCurrent = sessionState.currentSessionId
		const foreign = session({ id: 'sw-foreign', name: 'session-912', workspace_id: 'rootA' })
		const local = session({ id: 'sw-local', name: 'session-913', workspace_id: 'rootB' })
		sessionState.sessions.push(foreign, local)
		sessionState.currentSessionId = 'sw-foreign'
		try {
			await enterSessionMode()
			expect(sessionState.currentSessionId).toBe('sw-local')
			expect(goto).toHaveBeenCalledWith('/sessions?session_name=session-913', {
				replaceState: false
			})
		} finally {
			sessionState.sessions = sessionState.sessions.filter(
				(s) => s.id !== 'sw-foreign' && s.id !== 'sw-local'
			)
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})

	it('creates a fresh session in the active workspace when no in-family session exists', async () => {
		const restore = withTwoFamilies('rootB')
		const prevCurrent = sessionState.currentSessionId
		const foreign = session({ id: 'sw-only-foreign', name: 'session-914', workspace_id: 'rootA' })
		sessionState.sessions.push(foreign)
		sessionState.currentSessionId = 'sw-only-foreign'
		let createdId: string | undefined
		try {
			await enterSessionMode()
			createdId = sessionState.currentSessionId
			expect(createdId).not.toBe('sw-only-foreign')
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.pending_workspace_id).toBe('rootB')
			expect(created?.transient).toBe(true)
		} finally {
			sessionState.sessions = sessionState.sessions.filter(
				(s) => s.id !== 'sw-only-foreign' && s.id !== createdId
			)
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})
})

describe('enterSessionMode — seedFromEditor', () => {
	beforeEach(() => {
		vi.mocked(goto).mockClear()
		// Drop any registration a prior test left behind.
		registerEditorSessionSource(undefined)
	})

	it('opens the current editor item in a fresh session preview when a source is registered', async () => {
		const restore = withTwoFamilies('rootA')
		const prevCurrent = sessionState.currentSessionId
		const beforeOpen = vi.fn().mockResolvedValue(undefined)
		const cleanup = registerEditorSessionSource({
			target: { kind: 'flow', path: 'u/me/draft_x' },
			workspaceId: 'rootA',
			beforeOpen
		})
		let createdId: string | undefined
		try {
			await enterSessionMode({ seedFromEditor: true })
			// Persist ran, and we opened a fresh session scoped to the editor workspace.
			expect(beforeOpen).toHaveBeenCalledTimes(1)
			createdId = sessionState.currentSessionId
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.pending_workspace_id).toBe('rootA')
			expect(goto).toHaveBeenCalledWith(expect.stringContaining('/sessions?session_name='))
		} finally {
			cleanup()
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})

	it('falls back to generic restore when no editor source is registered', async () => {
		const restore = withTwoFamilies('rootB')
		const prevCurrent = sessionState.currentSessionId
		const local = session({ id: 'sw-seed-fallback', name: 'session-920', workspace_id: 'rootB' })
		sessionState.sessions.push(local)
		sessionState.currentSessionId = 'sw-seed-fallback'
		try {
			await enterSessionMode({ seedFromEditor: true })
			expect(sessionState.currentSessionId).toBe('sw-seed-fallback')
			expect(goto).toHaveBeenCalledWith('/sessions?session_name=session-920', {
				replaceState: false
			})
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== 'sw-seed-fallback')
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})

	it('ignores a registered source when seedFromEditor is not set (reconcile path)', async () => {
		const restore = withTwoFamilies('rootB')
		const prevCurrent = sessionState.currentSessionId
		const local = session({ id: 'sw-no-seed', name: 'session-921', workspace_id: 'rootB' })
		sessionState.sessions.push(local)
		sessionState.currentSessionId = 'sw-no-seed'
		const cleanup = registerEditorSessionSource({
			target: { kind: 'flow', path: 'u/me/draft_y' },
			workspaceId: 'rootB',
			beforeOpen: vi.fn()
		})
		try {
			await enterSessionMode()
			expect(sessionState.currentSessionId).toBe('sw-no-seed')
		} finally {
			cleanup()
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== 'sw-no-seed')
			sessionState.currentSessionId = prevCurrent
			restore()
		}
	})
})
