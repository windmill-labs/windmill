import { describe, expect, it, vi } from 'vitest'

vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: { parse: (value: string) => ({ toString: () => value }) },
	MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 }
}))
vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))
vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({ default: () => ({}) }))
vi.mock('$lib/components/vscode', () => ({}))

import { ResourceService } from '$lib/gen'
import { editCodeToolWithDiff, searchResourceTypes } from './core'

describe('searchResourceTypes', () => {
	const syncedNames = Array.from({ length: 30 }, (_, i) => `type_${i}`)

	// The similarity index trails the table by up to a day, and builds without embeddings
	// have none, so a type the query names must still be found.
	it('finds a type the query names when the similarity search misses it', async () => {
		vi.spyOn(ResourceService, 'queryResourceTypes').mockResolvedValue([])
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue([...syncedNames, 'stripe'])
		vi.spyOn(ResourceService, 'getResourceType').mockResolvedValue({
			name: 'stripe',
			schema: { type: 'object' }
		})

		const { resourceTypes, note } = await searchResourceTypes('Stripe payments', 'ws', 5)

		expect(resourceTypes.map((rt) => rt.name)).toEqual(['stripe'])
		expect(note).toBeUndefined()
	})

	// An empty result from a failed search says nothing about whether a type exists: read as
	// "none", it sends the model to define a type the instance already has.
	it('hands back the type names when the similarity search fails', async () => {
		vi.spyOn(ResourceService, 'queryResourceTypes').mockRejectedValue(new Error('500'))
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue([
			...syncedNames,
			'postgresql'
		])

		const { resourceTypes, note } = await searchResourceTypes('postgres database', 'ws', 5)

		expect(resourceTypes).toEqual([])
		expect(note).toContain('postgresql')
		expect(note).not.toContain('type_0')
	})

	it('keeps every type the query names ahead of the other matches', async () => {
		vi.spyOn(ResourceService, 'queryResourceTypes').mockResolvedValue(
			['a', 'b', 'c', 'd', 'stripe'].map((name) => ({ name, score: 0.8 }))
		)
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue([
			...syncedNames,
			'stripe',
			'github'
		])
		vi.spyOn(ResourceService, 'getResourceType').mockResolvedValue({ name: 'github', schema: {} })

		const { resourceTypes } = await searchResourceTypes('stripe github', 'ws', 5)

		expect(resourceTypes.map((rt) => rt.name)).toEqual(['stripe', 'github', 'a', 'b', 'c'])
	})

	it('notes an instance that never synced with the hub', async () => {
		vi.spyOn(ResourceService, 'queryResourceTypes').mockResolvedValue([])
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue([
			'ai_skill',
			'ai_instruction'
		])

		const { note } = await searchResourceTypes('stripe', 'ws', 5)

		expect(note).toContain('never synced')
	})
})

describe('editCodeToolWithDiff', () => {
	it('records the complete script diff after applying replacements', async () => {
		const statuses: unknown[] = []
		const applyCode = vi.fn(async () => {})

		await editCodeToolWithDiff.fn({
			args: { diffs: [{ old_string: 'return 1', new_string: 'return 2' }] },
			helpers: {
				getScriptOptions: () => ({
					code: 'export function main() {\n\treturn 1\n}',
					lang: 'bun',
					path: 'f/example',
					args: {}
				}),
				applyCode
			},
			toolCallbacks: {
				setToolStatus: (_toolId, status) => statuses.push(status)
			},
			toolId: 'edit-1'
		})

		expect(applyCode).toHaveBeenCalledTimes(2)
		expect(statuses.at(-1)).toMatchObject({
			codeDiff: {
				before: 'export function main() {\n\treturn 1\n}',
				after: 'export function main() {\n\treturn 2\n}',
				lang: 'typescript'
			}
		})
	})
})
