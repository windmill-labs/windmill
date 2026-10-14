import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { earlyReturnCandidates } from './earlyReturnCandidates'

function identity(id: string): FlowModule {
	return { id, value: { type: 'identity' } }
}

describe('earlyReturnCandidates', () => {
	it('offers every step reached through branches, but not the steps inside loops', () => {
		const modules: FlowModule[] = [
			identity('a'),
			{
				id: 'b',
				value: {
					type: 'branchone',
					default: [identity('c')],
					branches: [
						{
							expr: 'true',
							modules: [
								identity('d'),
								{
									id: 'e',
									value: {
										type: 'branchone',
										default: [],
										branches: [{ expr: 'true', modules: [identity('f')] }]
									}
								},
								{
									id: 'g',
									value: { type: 'branchall', branches: [{ modules: [identity('h')] }] }
								},
								{
									id: 'i',
									value: {
										type: 'forloopflow',
										iterator: { type: 'javascript', expr: '[]' },
										skip_failures: false,
										modules: [identity('j')]
									}
								}
							]
						}
					]
				}
			},
			{ id: 'k', value: { type: 'branchall', branches: [{ modules: [identity('l')] }] } },
			{
				id: 'm',
				value: {
					type: 'forloopflow',
					iterator: { type: 'javascript', expr: '[]' },
					skip_failures: false,
					modules: [identity('n')]
				}
			}
		]

		expect(earlyReturnCandidates(modules)).toEqual([
			{ id: 'a', deterministic: true },
			{ id: 'b', deterministic: true },
			{ id: 'c', deterministic: false },
			{ id: 'd', deterministic: false },
			{ id: 'e', deterministic: false },
			{ id: 'f', deterministic: false },
			{ id: 'g', deterministic: false },
			{ id: 'h', deterministic: false },
			{ id: 'i', deterministic: false },
			{ id: 'k', deterministic: true },
			{ id: 'l', deterministic: true },
			{ id: 'm', deterministic: true }
		])
	})
})
