import { describe, expect, it } from 'vitest'

import { findDefaultForkBase } from './workspaceHierarchy'
import type { UserWorkspace } from '../stores'

function ws(id: string, parent?: string, extra: Partial<UserWorkspace> = {}): UserWorkspace {
	return { id, name: id, username: 'u', parent_workspace_id: parent, disabled: false, ...extra }
}

const root = ws('prod')
const dev = ws('devws', 'prod', { is_dev_workspace: true })
const forkOfDev = ws('wm-fork-a', 'devws')
const forkOfRoot = ws('wm-fork-b', 'prod')
const family = [root, dev, forkOfDev, forkOfRoot]

describe('findDefaultForkBase', () => {
	it('bases a fork on the dev workspace from inside its subtree', () => {
		expect(findDefaultForkBase('devws', family)?.id).toBe('devws')
		expect(findDefaultForkBase('wm-fork-a', family)?.id).toBe('devws')
	})

	it('bases a fork on the root outside the dev subtree', () => {
		expect(findDefaultForkBase('prod', family)?.id).toBe('prod')
		expect(findDefaultForkBase('wm-fork-b', family)?.id).toBe('prod')
	})

	it('skips a dev workspace the user is disabled in', () => {
		const disabledDev = [root, { ...dev, disabled: true }, forkOfDev]
		expect(findDefaultForkBase('wm-fork-a', disabledDev)?.id).toBe('prod')
	})
})
