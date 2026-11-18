import { describe, it, expect } from 'vitest'
import { groupItems, type ItemType } from './treeViewUtils'

const item = (path: string) => ({ path }) as unknown as ItemType

describe('groupItems', () => {
	it('keeps a path outside u/ and f/ as a top-level leaf after users and folders', () => {
		const grouped = groupItems([item('g/all/hub_sync'), item('f/ops/a'), item('u/admin/b')])
		expect(grouped.map((n) => ('username' in n ? 'u' : 'folderName' in n ? 'f' : n.path))).toEqual([
			'u',
			'f',
			'g/all/hub_sync'
		])
	})
})
