import { describe, it, expect } from 'vitest'
import { scriptLangToEditorLang } from '$lib/scripts'
import { computeModelPath, computeModelUri } from './monacoUri'

// Mirrors what Editor.svelte does: path -> filePath -> uri.
function uriFor(path: string | undefined, scriptLang: string) {
	const filePath = computeModelPath(path, scriptLang)
	return computeModelUri(filePath, scriptLang, scriptLangToEditorLang(scriptLang as any))
}

describe('computeModelUri', () => {
	it.each([
		['u/admin/my_script', 'bun', 'file:///u/admin/my_script.ts'],
		['u/admin/my_script', 'bunnative', 'file:///u/admin/my_script.ts'],
		['u/admin/my_script', 'nativets', 'file:///u/admin/my_script.ts'],
		['u/admin/component', 'tsx', 'file:///u/admin/component.tsx'],
		['u/admin/component', 'jsx', 'file:///u/admin/component.js'],
		['u/admin/script', 'javascript', 'file:///u/admin/script.js'],
		// flow module and raw-app runnable derive their path as <item path>/<id>
		['f/flows/myflow/a', 'bun', 'file:///f/flows/myflow/a.ts'],
		['u/admin/myapp/backend_1', 'bun', 'file:///u/admin/myapp/backend_1.ts'],
		['/u/admin/leading_slash', 'bun', 'file:///u/admin/leading_slash.ts'],
		// a path that already carries an extension keeps it verbatim
		['u/admin/lib.ts', 'bun', 'file:///u/admin/lib.ts']
	])('%s (%s) -> %s', (path, scriptLang, expected) => {
		expect(uriFor(path, scriptLang)).toBe(expected)
	})

	it.each(['deno', 'go', 'python3'])(
		'randomizes the path and namespaces %s under /tmp/monaco',
		(scriptLang) => {
			const uri = uriFor('u/admin/my_script', scriptLang)
			expect(uri.startsWith('file:///tmp/monaco/')).toBe(true)
			expect(uri).not.toContain('my_script')
			expect(uri).not.toBe(uriFor('u/admin/my_script', scriptLang))
		}
	)

	it('falls back to a random path when none is given', () => {
		expect(computeModelPath(undefined, 'bun')).not.toBe('')
		expect(computeModelPath('', 'bun')).not.toBe('')
	})
})
