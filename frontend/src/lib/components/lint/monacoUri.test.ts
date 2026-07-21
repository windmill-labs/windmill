import { describe, it, expect } from 'vitest'
import { scriptLangToEditorLang } from '$lib/scripts'
import { computeModelPath, computeModelUri, computeOwnedUri, OWNED_ROOT } from './monacoUri'

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

describe('computeOwnedUri', () => {
	const cell = (
		over: Partial<{ workspace: string; itemKind: string; storagePath: string }> = {}
	) => ({
		workspace: 'guilhem',
		itemKind: 'script',
		storagePath: 'u/admin/foo',
		...over
	})

	it.each([
		[cell(), undefined, 'bun', 'file:///__wmlint__/guilhem/script/u/admin/foo.ts'],
		// a flow module / app runnable is a sub-path within the cell
		[
			cell({ itemKind: 'flow', storagePath: 'u/admin/flow' }),
			'a',
			'bun',
			'file:///__wmlint__/guilhem/flow/u/admin/flow/a.ts'
		],
		[
			cell({ itemKind: 'app', storagePath: 'u/admin/app' }),
			'runnable_1',
			'bun',
			'file:///__wmlint__/guilhem/app/u/admin/app/runnable_1.ts'
		],
		// an app frontend file keeps its extension
		[
			cell({ itemKind: 'app', storagePath: 'u/admin/app' }),
			'index.tsx',
			'tsx',
			'file:///__wmlint__/guilhem/app/u/admin/app/index.tsx'
		],
		// a fork is a different cell → a different owned URI
		[
			cell({ workspace: 'wm-fork-x' }),
			undefined,
			'bun',
			'file:///__wmlint__/wm-fork-x/script/u/admin/foo.ts'
		]
	])('%o + %s (%s) -> %s', (c, subPath, scriptLang, expected) => {
		expect(computeOwnedUri(c, subPath, scriptLang, scriptLangToEditorLang(scriptLang as any))).toBe(
			expected
		)
	})

	// The load-bearing invariant: an owned URI can never equal the editor URI for the same
	// code, so a headless model can never occupy a model an editor owns.
	it('is always disjoint from the editor URI', () => {
		for (const scriptLang of ['bun', 'bunnative', 'nativets', 'tsx', 'jsx', 'javascript']) {
			const editorLang = scriptLangToEditorLang(scriptLang as any)
			const owned = computeOwnedUri(cell(), undefined, scriptLang, editorLang)
			const editor = computeModelUri('u/admin/foo', scriptLang, editorLang)
			expect(owned).not.toBe(editor)
			expect(owned.startsWith(`file:///${OWNED_ROOT}/`)).toBe(true)
			expect(editor.startsWith(`file:///${OWNED_ROOT}/`)).toBe(false)
		}
	})

	it('separates the same path across workspaces (no cross-workspace collision)', () => {
		const a = computeOwnedUri(cell({ workspace: 'guilhem' }), undefined, 'bun', 'typescript')
		const b = computeOwnedUri(cell({ workspace: 'wm-fork-x' }), undefined, 'bun', 'typescript')
		expect(a).not.toBe(b)
	})
})
