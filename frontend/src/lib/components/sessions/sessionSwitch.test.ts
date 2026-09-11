import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
	enterSessionMode,
	enterSessionModeFromNav,
	openSourceInSession,
	rememberNavRoute,
	startSessionWithPrompt,
	takeNewSessionSeed
} from './sessionSwitch.svelte'
import {
	peekSessionAutoSend,
	sessionState,
	takeSessionAutoSend,
	type Session
} from './sessionState.svelte'
import { usersWorkspaceStore, workspaceStore, type UserWorkspace } from '$lib/stores'

vi.mock('$lib/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }))
import { goto } from '$lib/navigation'

// Seeding a preview tab dynamically imports the runtime, whose graph reaches
// monaco (hence that import being dynamic in the first place) and cannot load
// under node.
vi.mock('./sessionRuntime.svelte', () => ({ resetSessionPreviewTabs: vi.fn() }))
import { resetSessionPreviewTabs } from './sessionRuntime.svelte'
import { registerMountedOpenInSessionHandoff } from './openInSessionContext'

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

describe('openSourceInSession', () => {
	// The wrapper exists so an imperative caller cannot route before the source has
	// persisted the draft the preview loads.
	it('runs beforeOpen before routing, and lets overrides win over the source', async () => {
		vi.mocked(goto).mockClear()
		const order: string[] = []
		vi.mocked(goto).mockImplementation((async () => {
			order.push('goto')
		}) as never)
		const prevCurrent = sessionState.currentSessionId
		let createdId: string | undefined
		try {
			await openSourceInSession(
				{
					target: { kind: 'script', path: 'u/me/s' },
					beforeOpen: () => {
						order.push('beforeOpen')
					},
					seedPrompt: 'from source'
				},
				{ seedPrompt: 'from override' }
			)
			createdId = sessionState.currentSessionId
			expect(order).toEqual(['beforeOpen', 'goto'])
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.draftPrompt).toBe('from override')
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
			vi.mocked(goto).mockReset()
			vi.mocked(goto).mockResolvedValue(undefined as never)
		}
	})
})

// The auto-send intent must be claimable exactly once: SessionWrapper mounts per
// session and can remount, and a second claim would re-fire the same prompt as a
// duplicate turn.
describe('auto-send intent', () => {
	it('is carried by the hand-off and consumed by the first claim only', async () => {
		const prevCurrent = sessionState.currentSessionId
		let createdId: string | undefined
		try {
			await openSourceInSession({
				target: { kind: 'script', path: 'u/me/s' },
				seedPrompt: 'fix it',
				autoSend: true
			})
			createdId = sessionState.currentSessionId!
			expect(peekSessionAutoSend(createdId)).toBe(true)
			expect(takeSessionAutoSend(createdId)).toBe(true)
			expect(takeSessionAutoSend(createdId)).toBe(false)
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
		}
	})

	// `goto` resolves even when a beforeNavigate cancels it, so an abandoned
	// hand-off leaves the session armed. The claim must go stale rather than fire
	// a turn at whatever the user is doing whenever that session next opens.
	it('refuses a claim left over from a hand-off that never landed', async () => {
		const prevCurrent = sessionState.currentSessionId
		let createdId: string | undefined
		try {
			await openSourceInSession({
				target: { kind: 'script', path: 'u/me/s' },
				seedPrompt: 'fix it',
				autoSend: true
			})
			createdId = sessionState.currentSessionId!
			const s = sessionState.sessions.find((x) => x.id === createdId)!
			s.autoSendDraftAt = Date.now() - 10 * 60_000
			// The composer asks first, and must be told to show the prompt — blanking
			// it for an intent that is never honoured loses the text altogether.
			expect(peekSessionAutoSend(createdId)).toBe(false)
			expect(takeSessionAutoSend(createdId)).toBe(false)
			// Still cleared: a stale intent has no other consumer to leave it for.
			expect(s.autoSendDraftAt).toBeUndefined()
			expect(s.draftPrompt).toBe('fix it')
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
		}
	})

	it('is left unset when the caller only seeds the composer', async () => {
		const prevCurrent = sessionState.currentSessionId
		let createdId: string | undefined
		try {
			await openSourceInSession({
				target: { kind: 'script', path: 'u/me/s' },
				seedPrompt: 'pick inputs and run'
			})
			createdId = sessionState.currentSessionId!
			expect(takeSessionAutoSend(createdId)).toBe(false)
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
		}
	})
})

describe('startSessionWithPrompt', () => {
	beforeEach(() => {
		vi.mocked(goto).mockClear()
	})

	// The hand-off must not re-pick the workspace: createSession already steers off
	// a root the user cannot deploy to onto its dev, and overwriting that with the
	// raw current workspace lands the session somewhere it cannot edit.
	it('seeds the composer and keeps createSession’s workspace choice', async () => {
		const prevUsers = get(usersWorkspaceStore)
		const prevWs = get(workspaceStore)
		const prevCurrent = sessionState.currentSessionId
		usersWorkspaceStore.set({
			email: 't@t',
			workspaces: [ws('prod'), { ...ws('prod-dev', 'prod'), is_dev_workspace: true }]
		} as never)
		workspaceStore.set('prod')
		let createdId: string | undefined
		try {
			await startSessionWithPrompt('list my flows')
			createdId = sessionState.currentSessionId
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.draftPrompt).toBe('list my flows')
			expect(created?.pending_workspace_id).toBe('prod-dev')
		} finally {
			sessionState.sessions = sessionState.sessions.filter((s) => s.id !== createdId)
			sessionState.currentSessionId = prevCurrent
			usersWorkspaceStore.set(prevUsers)
			workspaceStore.set(prevWs)
		}
	})
})

describe('takeNewSessionSeed', () => {
	it('offers the item editor the user came from, once per arrival', () => {
		rememberNavRoute('/flows/edit/u/me/my_flow?workspace=ws')
		expect(takeNewSessionSeed()).toEqual({
			url: '/flows/edit/u/me/my_flow?workspace=ws',
			route: { kind: 'flow', raw_app: false, itemPath: 'u/me/my_flow' }
		})
		// A dismissed or taken offer is not repeated until the user leaves again.
		expect(takeNewSessionSeed()).toBeUndefined()
		rememberNavRoute('/apps_raw/edit/f/team/dashboard?workspace=ws')
		expect(takeNewSessionSeed()?.route).toEqual({
			kind: 'app',
			raw_app: true,
			itemPath: 'f/team/dashboard'
		})
	})

	it('offers nothing for a non-item page', () => {
		rememberNavRoute('/runs?workspace=ws')
		expect(takeNewSessionSeed()).toBeUndefined()
	})
})

describe('enterSessionModeFromNav', () => {
	const HOUR = 60 * 60 * 1000
	beforeEach(() => {
		vi.mocked(goto).mockClear()
		vi.mocked(resetSessionPreviewTabs).mockClear()
	})

	// One session in the active family (rootA), as the one the rail would resume.
	// Restores everything the test touched, the module-level nav route included,
	// and drops whatever session the call under test created.
	function withResumable(over: Partial<Session>): { restore: () => void } {
		const restoreFamilies = withTwoFamilies('rootA')
		const prevCurrent = sessionState.currentSessionId
		const before = new Set(sessionState.sessions.map((s) => s.id))
		const s = session({ workspace_id: 'rootA', ...over })
		sessionState.sessions.push(s)
		sessionState.currentSessionId = s.id
		return {
			restore: () => {
				sessionState.sessions = sessionState.sessions.filter((x) => before.has(x.id))
				sessionState.currentSessionId = prevCurrent
				rememberNavRoute('/')
				restoreFamilies()
			}
		}
	}

	it('resumes a recently active session even when coming from an item', async () => {
		const { restore } = withResumable({
			id: 'nav-recent',
			name: 'session-921',
			lastActivityAt: Date.now() - HOUR / 2
		})
		try {
			rememberNavRoute('/flows/edit/u/me/f?workspace=rootA')
			await enterSessionModeFromNav()
			expect(sessionState.currentSessionId).toBe('nav-recent')
			expect(resetSessionPreviewTabs).not.toHaveBeenCalled()
		} finally {
			restore()
		}
	})

	it('starts a session on the item, by route, instead of resuming one idle for hours', async () => {
		const { restore } = withResumable({
			id: 'nav-stale',
			name: 'session-922',
			lastActivityAt: Date.now() - 3 * HOUR
		})
		try {
			rememberNavRoute('/flows/edit/u/me/f?workspace=forkA')
			await enterSessionModeFromNav()
			const createdId = sessionState.currentSessionId
			expect(createdId).not.toBe('nav-stale')
			expect(resetSessionPreviewTabs).toHaveBeenCalledWith(
				createdId,
				'/flows/edit/u/me/f?workspace=forkA'
			)
			// The route's workspace, not createSession's pick for the active one.
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.pending_workspace_id).toBe('forkA')
			// The arrival opened the item itself, so "New session" does not offer it.
			expect(takeNewSessionSeed()).toBeUndefined()
		} finally {
			restore()
		}
	})

	// The editor's hand-off is what its own "Open in AI session" button uses: its
	// beforeOpen persists the draft the preview loads, and it names the item's
	// workspace. The rail must take it over opening the route as last persisted.
	it('hands off through the mounted editor of the item, running its beforeOpen first', async () => {
		const { restore } = withResumable({
			id: 'nav-stale-editor',
			name: 'session-924',
			lastActivityAt: Date.now() - 3 * HOUR
		})
		const order: string[] = []
		vi.mocked(goto).mockImplementation((async () => {
			order.push('goto')
		}) as never)
		// A script editor mounted alongside (a flow's drawer) must not be taken
		// for the page's own item.
		const unregisterScript = registerMountedOpenInSessionHandoff({
			source: () => ({ target: { kind: 'script', path: 'u/me/s' }, workspaceId: 'rootB' })
		})
		const unregisterFlow = registerMountedOpenInSessionHandoff({
			source: () => ({
				target: { kind: 'flow', path: 'u/me/f' },
				workspaceId: 'forkA',
				beforeOpen: () => {
					order.push('beforeOpen')
				}
			})
		})
		try {
			rememberNavRoute('/flows/edit/u/me/f?workspace=rootA')
			await enterSessionModeFromNav()
			const createdId = sessionState.currentSessionId
			expect(createdId).not.toBe('nav-stale-editor')
			expect(order).toEqual(['beforeOpen', 'goto'])
			expect(resetSessionPreviewTabs).toHaveBeenCalledWith(
				createdId,
				expect.stringMatching(/\/flows\/edit\/u\/me\/f$/)
			)
			const created = sessionState.sessions.find((s) => s.id === createdId)
			expect(created?.pending_workspace_id).toBe('forkA')
		} finally {
			unregisterFlow()
			unregisterScript()
			vi.mocked(goto).mockReset()
			vi.mocked(goto).mockResolvedValue(undefined as never)
			restore()
		}
	})

	it('resumes a stale session when coming from a non-item page', async () => {
		const { restore } = withResumable({
			id: 'nav-stale-runs',
			name: 'session-923',
			lastActivityAt: Date.now() - 3 * HOUR
		})
		try {
			rememberNavRoute('/runs?workspace=rootA')
			await enterSessionModeFromNav()
			expect(sessionState.currentSessionId).toBe('nav-stale-runs')
			expect(resetSessionPreviewTabs).not.toHaveBeenCalled()
		} finally {
			restore()
		}
	})
})
