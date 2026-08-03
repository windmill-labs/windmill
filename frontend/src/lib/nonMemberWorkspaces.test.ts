import { describe, expect, it, beforeEach } from 'vitest'
import { get } from 'svelte/store'
import {
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
	setNonMemberWorkspaces([])
})

describe('setNonMemberWorkspaces', () => {
	it('merges the resolved workspaces into userWorkspaces with their fork lineage', () => {
		setNonMemberWorkspaces([
			workspace('dev', { parent_workspace_id: 'prod', is_dev_workspace: true })
		])
		expect(get(userWorkspaces)).toMatchObject([
			{ id: 'dev', parent_workspace_id: 'prod', is_dev_workspace: true }
		])
	})

	it('never offers an archived workspace', () => {
		setNonMemberWorkspaces([workspace('archived-fork', { deleted: true })])
		expect(get(userWorkspaces)).toEqual([])
	})

	it('replaces the previous set rather than accumulating across workspace switches', () => {
		setNonMemberWorkspaces([workspace('fork-a')])
		setNonMemberWorkspaces([workspace('fork-b')])
		expect(get(userWorkspaces).map((w) => w.id)).toEqual(['fork-b'])
	})

	// Effects that resolve the current workspace depend on `userWorkspaces`; re-emitting
	// an unchanged set would re-run them, which re-resolves, which emits again.
	it('keeps the same object when the ids are unchanged', () => {
		setNonMemberWorkspaces([workspace('fork-a')])
		const first = get(nonMemberWorkspaces)
		setNonMemberWorkspaces([workspace('fork-a')])
		expect(get(nonMemberWorkspaces)).toBe(first)
	})

	it('does not duplicate a workspace the user is a member of', () => {
		usersWorkspaceStore.set({
			email: 'admin@windmill.dev',
			workspaces: [{ id: 'dev', name: 'dev', username: 'admin', color: '', disabled: false }]
		} as any)
		setNonMemberWorkspaces([workspace('dev')])
		expect(get(userWorkspaces).map((w) => w.id)).toEqual(['dev'])
	})
})
