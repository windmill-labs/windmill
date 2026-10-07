import { describe, it, expect, vi, afterEach } from 'vitest'
import { get } from 'svelte/store'
import { DraftService } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { sessionState, type Session } from './sessionState.svelte'
import { moveSessionToFork } from './moveSessionToFork'

vi.mock('$lib/gen', async (orig) => {
	const actual = await orig<typeof import('$lib/gen')>()
	return {
		...actual,
		DraftService: { ...actual.DraftService, transferDrafts: vi.fn() }
	}
})

const items = [{ kind: 'script' as const, path: 'u/a/s' }]

function addSession(): Session {
	const s = { id: 'move-test', name: 'm', createdAt: 0, workspace_id: 'root_ws' } as Session
	sessionState.sessions.push(s)
	return sessionState.sessions.find((x) => x.id === s.id)!
}

afterEach(() => {
	const i = sessionState.sessions.findIndex((x) => x.id === 'move-test')
	if (i >= 0) sessionState.sessions.splice(i, 1)
})

describe('moveSessionToFork', () => {
	it('keeps the session on its workspace when the fork already has a draft of an item', async () => {
		vi.mocked(DraftService.transferDrafts).mockResolvedValue({
			copied: [],
			removed: [],
			conflicts: items
		})
		const s = addSession()
		const res = await moveSessionToFork(
			s.id,
			'root_ws',
			{ kind: 'existing', id: 'wm-fork-x' },
			{
				items,
				removeFromParent: true
			}
		)
		expect(res).toMatchObject({ ok: false, conflicts: items })
		expect(s.workspace_id).toBe('root_ws')
	})

	it('moves only the session, not the navigation workspace, once the drafts are copied', async () => {
		vi.mocked(DraftService.transferDrafts).mockResolvedValue({
			copied: items,
			removed: [],
			conflicts: []
		})
		const prev = get(workspaceStore)
		workspaceStore.set('root_ws')
		try {
			const s = addSession()
			const res = await moveSessionToFork(
				s.id,
				'root_ws',
				{ kind: 'existing', id: 'wm-fork-x' },
				{
					items,
					removeFromParent: false
				}
			)
			expect(res).toMatchObject({ ok: true, forkId: 'wm-fork-x', copied: 1 })
			expect(DraftService.transferDrafts).toHaveBeenCalledWith({
				workspace: 'root_ws',
				requestBody: { target_workspace: 'wm-fork-x', items, remove_from_source: false }
			})
			expect(s.workspace_id).toBe('wm-fork-x')
			expect(get(workspaceStore)).toBe('root_ws')
		} finally {
			workspaceStore.set(prev)
		}
	})
})
