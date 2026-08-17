import { describe, it, expect, vi } from 'vitest'

// Mock heavy transitive imports pulled in by AssetNode.svelte's instance script
vi.mock('monaco-editor', () => ({}))
vi.mock('$lib/components/meltComponents', () => ({ Tooltip: {} }))
vi.mock('../../../ExploreAssetButton.svelte', () => ({
	default: {},
	assetCanBeExplored: () => false
}))
vi.mock('$lib/components/icons/AssetGenericIcon.svelte', () => ({ default: {} }))
vi.mock('$lib/components/assets/AssetColumnBadges.svelte', () => ({ default: {} }))
vi.mock('./NodeWrapper.svelte', () => ({ default: {} }))

import { computeAssetNodes } from './AssetNode.svelte'

function nodeWithAssets(id: string, assets: any[]) {
	return { id, position: { x: 0, y: 0 }, data: { assets } }
}

describe('computeAssetNodes (WIN-1998)', () => {
	it('produces unique node and edge ids when a module lists the same asset twice', () => {
		// Two assets with identical kind+path (e.g. read twice, or r + rw) — both
		// display as inputs. Before the fix these collided on the same node id and
		// crashed SvelteFlow with `each_key_duplicate`.
		const dup = { kind: 'resource', path: 'f/foo/bar', access_type: 'r' }
		const { newAssetNodes, newAssetEdges } = computeAssetNodes([
			nodeWithAssets('moduleA', [{ ...dup }, { ...dup }])
		])

		const nodeIds = newAssetNodes.map((n) => n.id)
		expect(new Set(nodeIds).size).toBe(nodeIds.length)

		const edgeIds = newAssetEdges.map((e) => e.id)
		expect(new Set(edgeIds).size).toBe(edgeIds.length)
	})

	it('does not emit overflow edges when there is no overflow node (<=3 assets)', () => {
		const { newAssetNodes, newAssetEdges } = computeAssetNodes([
			nodeWithAssets('moduleB', [{ kind: 'resource', path: 'f/a/x', access_type: 'r' }])
		])

		// No overflow node should be created for a single asset...
		expect(newAssetNodes.some((n) => n.type === 'assetsOverflowed')).toBe(false)
		// ...and therefore no dangling edge should reference a missing overflow node.
		const nodeIdSet = new Set(newAssetNodes.map((n) => n.id).concat('moduleB'))
		for (const e of newAssetEdges) {
			expect(nodeIdSet.has(e.source as string)).toBe(true)
			expect(nodeIdSet.has(e.target as string)).toBe(true)
		}
	})
})
