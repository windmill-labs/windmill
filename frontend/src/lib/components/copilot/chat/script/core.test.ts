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
import {
	editCodeToolWithDiff,
	resolveResourceTypeSync,
	resourceTypeSyncBlock,
	searchResourceTypes
} from './core'

describe('searchResourceTypes', () => {
	it.each([
		[['ai_skill', 'ai_instruction'], true],
		[Array.from({ length: 30 }, (_, i) => `type_${i}`), false]
	])('notes only an instance that never synced with the hub (%#)', async (names, noted) => {
		vi.spyOn(ResourceService, 'queryResourceTypes').mockResolvedValue([
			{ name: 'ai_skill', score: 0.8 }
		])
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue(names)

		const { note } = await searchResourceTypes('stripe', 'ws', 5)

		expect(note?.includes('never synced') ?? false).toBe(noted)
	})
})

// The transition is what keeps the note out of the system prompt: a conversation that was
// told "not synced" has to be told once when that stops being true, and nobody else is.
describe('resolveResourceTypeSync', () => {
	const synced = Array.from({ length: 30 }, (_, i) => `type_${i}`)
	const told = (state: 'unsynced' | 'now_synced') => ({
		role: 'user',
		content: resourceTypeSyncBlock(state)
	})

	it.each([
		['unsynced instance', ['ai_skill'], [], 'unsynced'],
		['always synced', synced, [], undefined],
		['synced after an unsynced note', synced, [told('unsynced')], 'now_synced'],
		['already told it synced', synced, [told('unsynced'), told('now_synced')], undefined]
	] as const)('%s', async (label, names, history, expected) => {
		vi.spyOn(ResourceService, 'listResourceTypeNames').mockResolvedValue([...names])

		expect(await resolveResourceTypeSync(`ws-${label}`, history)).toBe(expected)
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
