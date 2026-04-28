import { describe, expect, it, vi } from 'vitest'
import { z } from 'zod'

vi.mock('monaco-editor', () => ({
	editor: {}
}))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined }
}))

vi.mock('$lib/components/flows/flowTree', () => ({
	findModuleInModules: () => undefined
}))

vi.mock('$lib/gen', () => ({
	ScriptService: {},
	FlowService: {},
	JobService: {}
}))

vi.mock('$lib/utils', () => ({
	emptyString: (value: string | undefined | null) => !value
}))

vi.mock('$lib/scripts', () => ({
	scriptLangToEditorLang: (language: string) => language
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: () => undefined
}))

vi.mock('@leeoniya/ufuzzy', () => ({
	default: class {
		search() {
			return [[], [], []]
		}
	}
}))

describe('createToolDef', () => {
	it('adds a top-level object type for composed object schemas', async () => {
		const { createToolDef } = await import('./shared')
		const toolDef = createToolDef(
			z.discriminatedUnion('kind', [
				z.object({ kind: z.literal('script'), path: z.string() }),
				z.object({ kind: z.literal('flow'), path: z.string() })
			]),
			'create_runnable_trigger',
			'Create a runnable trigger'
		)

		const parameters = toolDef.function.parameters
		expect(parameters).toBeDefined()
		expect(parameters?.type).toBe('object')
		expect(parameters?.anyOf).toHaveLength(2)
	})
})
