import { describe, it, expect, vi, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
	noteArrival,
	openSourceInSession,
	resumeAiSessions,
	returnToWorkspace,
	startSessionWithPrompt
} from './sessionSwitch.svelte'
import {
	homeDraft,
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

function ws(id: string, parent?: string): UserWorkspace {
	return { id, name: id, parent_workspace_id: parent } as unknown as UserWorkspace
}

function session(over: Partial<Session> & { id: string }): Session {
	return { createdAt: 0, ...over }
}

// Two families: rootA (with forkA) and rootB. Returns a restore fn.
function withTwoFamilies(activeWorkspace: string): () => void {
	const prevUsers = get(usersWorkspaceStore)
	const prevWs = get(workspaceStore)
	const prevCurrent = sessionState.currentSessionId
	usersWorkspaceStore.set({
		email: 't@t',
		workspaces: [ws('rootA'), ws('forkA', 'rootA'), ws('rootB')]
	} as never)
	workspaceStore.set(activeWorkspace)
	return () => {
		sessionState.sessions = sessionState.sessions.filter((s) => !s.id.startsWith('sw-'))
		sessionState.currentSessionId = prevCurrent
		usersWorkspaceStore.set(prevUsers)
		workspaceStore.set(prevWs)
	}
}

describe('the sidebar switch comes back where each side was left', () => {
	beforeEach(() => {
		vi.mocked(goto).mockClear()
	})

	it('resumes the session last opened, within the active family', async () => {
		const restore = withTwoFamilies('forkA')
		sessionState.sessions.push(
			session({ id: 'sw-opened', workspace_id: 'rootA', lastActivityAt: 1 }),
			session({ id: 'sw-recent', workspace_id: 'forkA', lastActivityAt: 2 }),
			session({ id: 'sw-foreign', workspace_id: 'rootB', lastActivityAt: 3 })
		)
		try {
			noteArrival(new URL('http://localhost/sessions?session=sw-opened'))
			await resumeAiSessions()
			expect(goto).toHaveBeenLastCalledWith('/sessions?session=sw-opened', {
				replaceState: false
			})
			// Another family's session would pull its scope into this one: the family's most
			// recently used session stands in.
			noteArrival(new URL('http://localhost/sessions?session=sw-foreign'))
			await resumeAiSessions()
			expect(goto).toHaveBeenLastCalledWith('/sessions?session=sw-recent', {
				replaceState: false
			})
		} finally {
			restore()
		}
	})

	it('reopens an unsent draft in Home’s composer', async () => {
		const restore = withTwoFamilies('rootA')
		sessionState.sessions.push(
			session({ id: 'sw-draft', pending_workspace_id: 'rootA', draftPrompt: 'half typed' })
		)
		// Switched from a workspace page, so Home is not already open.
		vi.stubGlobal('window', { location: { pathname: '/runs' } })
		try {
			noteArrival(new URL('http://localhost/sessions?session=sw-draft'))
			await resumeAiSessions()
			expect(goto).toHaveBeenLastCalledWith('/', { replaceState: false })
			expect(homeDraft.id).toBe('sw-draft')
		} finally {
			vi.unstubAllGlobals()
			homeDraft.id = undefined
			restore()
		}
	})

	it('returns to the workspace page left, moved by the picker’s rule after a switch', async () => {
		const restore = withTwoFamilies('rootA')
		try {
			noteArrival(new URL('http://localhost/runs?workspace=rootA'))
			// Home with the session list beside it is not a workspace page to come back to.
			noteArrival(new URL('http://localhost/sessions?session=sw-any'))
			noteArrival(new URL('http://localhost/'))
			await returnToWorkspace()
			expect(goto).toHaveBeenLastCalledWith('/runs?workspace=rootA')

			workspaceStore.set('rootB')
			await returnToWorkspace()
			expect(goto).toHaveBeenLastCalledWith('/runs?workspace=rootB')

			noteArrival(new URL('http://localhost/flows/edit/f/foo'))
			workspaceStore.set('rootA')
			await returnToWorkspace()
			expect(goto).toHaveBeenLastCalledWith('/')
		} finally {
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
