import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { earlyReturnCandidates } from './earlyReturnCandidates'

function identity(id: string): FlowModule {
	return { id, value: { type: 'identity' } }
}

describe('earlyReturnCandidates', () => {
	it('offers top-level nodes and nodes reached through branches to one only', () => {
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
			{ id: 'c', location: 'b › Default', deterministic: false },
			{ id: 'd', location: 'b › Branch 1', deterministic: false },
			{ id: 'e', location: 'b › Branch 1', deterministic: false },
			{ id: 'f', location: 'b › Branch 1 › e › Branch 1', deterministic: false },
			{ id: 'k', deterministic: true },
			{ id: 'm', deterministic: true }
		])
	})
})
