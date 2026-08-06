import { describe, expect, it, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
	clearNonMemberWorkspaces,
	nonMemberWorkspaces,
	setNonMemberWorkspaces,
	superadmin,
	userWorkspaces,
	usersWorkspaceStore
} from './stores'
import type { Workspace } from './gen'

function workspace(id: string, extra: Partial<Workspace> = {}): Workspace {
	return { id, name: id, owner: 'admin@windmill.dev', ...extra } as Workspace
}

beforeEach(() => {
	usersWorkspaceStore.set(undefined)
	superadmin.set(undefined)
	nonMemberWorkspaces.set(undefined)
})

describe('nonMemberWorkspaces', () => {
	it('merges the resolved workspaces into userWorkspaces with their fork lineage', () => {
		setNonMemberWorkspaces('dev', [
			workspace('dev', { parent_workspace_id: 'prod', is_dev_workspace: true }),
			workspace('prod')
		])
		expect(get(userWorkspaces)).toMatchObject([
			{ id: 'dev', parent_workspace_id: 'prod', is_dev_workspace: true },
			{ id: 'prod' }
		])
	})

	it('never offers an archived workspace, and still marks it resolved', () => {
		setNonMemberWorkspaces('archived-fork', [workspace('archived-fork', { deleted: true })])
		expect(get(userWorkspaces)).toEqual([])
		// Without this the layout re-fetches the archived workspace on every effect pass.
		expect(get(nonMemberWorkspaces)?.forWorkspace).toBe('archived-fork')
	})

	it('replaces the previous workspace set rather than accumulating across switches', () => {
		setNonMemberWorkspaces('fork-a', [workspace('fork-a')])
		setNonMemberWorkspaces('fork-b', [workspace('fork-b')])
		expect(get(userWorkspaces).map((w) => w.id)).toEqual(['fork-b'])
	})

	it('does not duplicate a workspace the user is a member of', () => {
		usersWorkspaceStore.set({
			email: 'admin@windmill.dev',
			workspaces: [{ id: 'dev', name: 'dev', username: 'admin', color: '', disabled: false }]
		} as any)
		setNonMemberWorkspaces('dev', [workspace('dev')])
		expect(get(userWorkspaces).map((w) => w.id)).toEqual(['dev'])
	})

	// The layout effect that fills this store also reads it, and its member path — every
	// ordinary user, every pass — clears it. So the empty state must be one Svelte won't
	// re-notify for: an object value always notifies, however equal, and the effect would
	// then invalidate itself forever.
	it('does not notify when clearing an already empty store', () => {
		let notifications = 0
		const unsubscribe = userWorkspaces.subscribe(() => notifications++)
		notifications = 0
		clearNonMemberWorkspaces()
		clearNonMemberWorkspaces()
		expect(notifications).toBe(0)
		unsubscribe()
	})
})
