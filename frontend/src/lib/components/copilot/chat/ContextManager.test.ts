import { beforeEach, describe, expect, it, vi } from 'vitest'

const { listResource } = vi.hoisted(() => ({
	listResource: vi.fn()
}))

vi.mock('$lib/gen', () => ({
	ResourceService: {
		listResource
	}
}))

vi.mock('$lib/scripts', () => ({
	scriptLangToEditorLang: (language: string) => language
}))

vi.mock('$lib/stores', () => ({
	SQLSchemaLanguages: ['postgresql']
}))

vi.mock('$lib/editorLangUtils', () => ({
	langToExt: () => 'ts'
}))

describe('ContextManager', () => {
	beforeEach(() => {
		listResource.mockReset()
	})

	const scriptOptions = {
		lang: 'bun' as const,
		code: 'export async function main() {}',
		error: undefined,
		args: {},
		path: 'f/scripts/example',
		diffMode: false
	}

	it('loads db resources when switching from global to script in the same workspace', async () => {
		const { default: ContextManager } = await import('./ContextManager.svelte')
		const manager = new ContextManager()
		listResource.mockResolvedValueOnce([{ path: 'f/db/main' }])

		manager.updateAvailableContextForGlobal('workspace-a', [])
		await manager.updateAvailableContext(scriptOptions, {}, 'workspace-a', true, [])

		expect(listResource).toHaveBeenCalledWith({
			workspace: 'workspace-a',
			resourceType: 'postgresql'
		})
		expect(manager.getAvailableContext()).toEqual(
			expect.arrayContaining([expect.objectContaining({ type: 'db', title: 'f/db/main' })])
		)
	})

	it('refreshes db resources after switching workspaces through global mode', async () => {
		const { default: ContextManager } = await import('./ContextManager.svelte')
		const manager = new ContextManager()
		listResource
			.mockResolvedValueOnce([{ path: 'f/db/workspace-a' }])
			.mockResolvedValueOnce([{ path: 'f/db/workspace-b' }])

		await manager.updateAvailableContext(scriptOptions, {}, 'workspace-a', true, [])
		manager.updateAvailableContextForGlobal('workspace-b', [])
		await manager.updateAvailableContext(scriptOptions, {}, 'workspace-b', true, [])

		expect(listResource).toHaveBeenCalledTimes(2)
		expect(listResource).toHaveBeenLastCalledWith({
			workspace: 'workspace-b',
			resourceType: 'postgresql'
		})
		expect(manager.getAvailableContext()).toEqual(
			expect.arrayContaining([expect.objectContaining({ type: 'db', title: 'f/db/workspace-b' })])
		)
		expect(manager.getAvailableContext()).not.toEqual(
			expect.arrayContaining([expect.objectContaining({ type: 'db', title: 'f/db/workspace-a' })])
		)
	})
})
