import { describe, expect, it } from 'vitest'

import { recentWorkspaceRoots } from './recentWorkspaces'
import type { UserWorkspace } from '../stores'

function ws(id: string, parent?: string): UserWorkspace {
	return { id, name: id, username: 'u', parent_workspace_id: parent, disabled: false }
}

const roots = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'].map((id) => ws(id))
const workspaces = [...roots, ws('wm-fork-x', 'c'), ws('wm-fork-y', 'wm-fork-x')]

describe('recentWorkspaceRoots', () => {
	it('counts a fork visit for its root and keeps the first families visited', () => {
		const visited = ['wm-fork-y', 'gone', 'c', 'e', 'a', 'b']
		expect([...recentWorkspaceRoots(visited, workspaces)]).toEqual(['c', 'e', 'a'])
	})

	it('leaves a short list without a recent group', () => {
		expect(recentWorkspaceRoots(['a'], roots).size).toBe(0)
	})
})
