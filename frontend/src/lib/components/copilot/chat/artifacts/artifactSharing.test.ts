import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/base', () => ({ base: '' }))

import { shareWorkspaceId } from './artifactSharing'

describe('shareWorkspaceId', () => {
	it('shares a fork session into the topmost workspace the user still belongs to', () => {
		const workspaces = [
			{ id: 'prod' },
			{ id: 'wm-fork-a', parent_workspace_id: 'prod' },
			{ id: 'wm-fork-b', parent_workspace_id: 'wm-fork-a' }
		]
		expect(shareWorkspaceId('wm-fork-b', workspaces)).toBe('prod')
	})

	it('stops below a parent the user is not a member of', () => {
		const workspaces = [{ id: 'wm-fork-a', parent_workspace_id: 'prod' }]
		expect(shareWorkspaceId('wm-fork-a', workspaces)).toBe('wm-fork-a')
	})
})
