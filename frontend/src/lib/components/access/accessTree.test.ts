import { describe, expect, it } from 'vitest'
import type { PrincipalAccess } from '$lib/gen'
import { buildAccessTree } from './accessTree'

const item = (path: string, reasons: PrincipalAccess['items'][number]['reasons']) => ({
	kind: 'script',
	path,
	write: false,
	reasons
})

describe('buildAccessTree', () => {
	it('marks a folder or space whole only when it is held itself', () => {
		const tree = buildAccessTree({
			principal: 'u/alice',
			is_admin: false,
			groups: ['ops'],
			folders: [{ name: 'reporting', write: false, owner: false, via: ['g/ops'] }],
			items: [
				item('u/bob/shared_note', [{ type: 'group', group: 'ops', write: true }]),
				item('g/ops/oncall_page', [{ type: 'group_path', group: 'ops' }]),
				item('f/reporting/archive/q3_rollup', [{ type: 'folder', folder: 'reporting' }]),
				item('f/ledger/close_month', [{ type: 'group', group: 'ops', write: false }]),
				item('u/alice/scratch', [{ type: 'own' }])
			]
		})

		expect(tree.map((n) => [n.key, n.whole])).toEqual([
			['u/alice', true],
			['f/ledger', false],
			['f/reporting', true],
			['g/ops', true],
			['u/bob', false]
		])
		expect(tree[2].subfolders.map((s) => s.key)).toEqual(['f/reporting/archive'])
	})
})
