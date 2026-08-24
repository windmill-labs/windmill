import { describe, expect, it } from 'vitest'

import {
	devWorkspacesInChainAbove,
	findDefaultForkBase,
	findWorkspaceAncestors,
	findWorkspaceRoot
} from './workspaceHierarchy'
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

const nestedDev = ws('stgws', 'devws', { is_dev_workspace: true })
const forkOfNestedDev = ws('wm-fork-c', 'stgws')
const nestedFamily = [...family, nestedDev, forkOfNestedDev]

describe('findWorkspaceRoot', () => {
	it('walks to the family root through a single dev workspace', () => {
		expect(findWorkspaceRoot('devws', family)?.id).toBe('prod')
		expect(findWorkspaceRoot('wm-fork-a', family)?.id).toBe('prod')
	})

	it('stops at the parent dev workspace of a dev of a dev', () => {
		expect(findWorkspaceRoot('stgws', nestedFamily)?.id).toBe('devws')
		expect(findWorkspaceRoot('wm-fork-c', nestedFamily)?.id).toBe('devws')
	})

	it('stops at the highest reachable ancestor', () => {
		expect(findWorkspaceRoot('devws', [dev, forkOfDev])?.id).toBe('devws')
	})
})

describe('findWorkspaceAncestors', () => {
	// The attach-candidate cycle filter depends on this NOT stopping where findWorkspaceRoot does.
	it('walks past a dev-of-dev boundary to the true root', () => {
		expect(findWorkspaceAncestors('wm-fork-c', nestedFamily).map((w) => w.id)).toEqual([
			'stgws',
			'devws',
			'prod'
		])
	})
})

describe('devWorkspacesInChainAbove', () => {
	it('collects the dev workspaces at and above the prod side', () => {
		expect(devWorkspacesInChainAbove('stgws', nestedFamily).map((w) => w.id)).toEqual([
			'stgws',
			'devws'
		])
		expect(devWorkspacesInChainAbove('prod', nestedFamily)).toEqual([])
	})
})
