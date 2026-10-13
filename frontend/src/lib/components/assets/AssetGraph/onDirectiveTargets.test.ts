import { describe, expect, it } from 'vitest'
import { onDirectiveTargets } from './onDirectiveTargets'

describe('onDirectiveTargets', () => {
	it('leaves out what the script feeds, directly or not, and what already triggers it', () => {
		const nodes = [
			{ id: 'script:a', type: 'runnable' },
			{ id: 'script:b', type: 'runnable' },
			{ id: 'asset:out', type: 'asset' },
			{ id: 'asset:later', type: 'asset' },
			{ id: 'asset:in', type: 'asset' },
			{ id: 'asset:free', type: 'asset' }
		]
		const edges = [
			{ source: 'script:a', target: 'asset:out', kind: 'lineage-write' },
			{ source: 'asset:out', target: 'script:b', kind: 'trigger-asset' },
			{ source: 'script:b', target: 'asset:later', kind: 'lineage-write' },
			{ source: 'asset:in', target: 'script:a', kind: 'trigger-asset' }
		]
		expect([...onDirectiveTargets(nodes, edges, 'script:a')]).toEqual(['asset:free'])
	})
})
