import { describe, expect, it } from 'vitest'
import {
	parseRawAppDiff,
	rawAppDiffToItems,
	RAW_APP_METADATA_PATH,
	type RawAppDiffEntry
} from './rawAppDiffUtils'

function byPath(entries: RawAppDiffEntry[], path: string): RawAppDiffEntry | undefined {
	return entries.find((e) => e.path === path)
}

describe('parseRawAppDiff — files', () => {
	it('detects added, removed and modified files, omits unchanged', () => {
		const original = {
			files: { 'index.html': '<h1>hi</h1>', 'styles.css': 'body{}', 'gone.js': 'x' }
		}
		const current = {
			files: { 'index.html': '<h1>hello</h1>', 'styles.css': 'body{}', 'new.ts': 'y' }
		}
		const entries = parseRawAppDiff(original, current)
		const paths = entries.map((e) => e.path).sort()
		// styles.css unchanged → omitted
		expect(paths).toEqual(['gone.js', 'index.html', 'new.ts'])

		expect(byPath(entries, 'index.html')?.status).toBe('modified')
		expect(byPath(entries, 'index.html')?.lang).toBe('html')
		expect(byPath(entries, 'gone.js')?.status).toBe('removed')
		expect(byPath(entries, 'new.ts')?.status).toBe('added')
		expect(byPath(entries, 'new.ts')?.lang).toBe('typescript')
	})

	it('carries original/current content on the right sides', () => {
		const entries = parseRawAppDiff(
			{ files: { 'a.txt': 'old', 'b.txt': 'keep' } },
			{ files: { 'a.txt': 'new', 'b.txt': 'keep' } }
		)
		const a = byPath(entries, 'a.txt')!
		expect(a.original).toBe('old')
		expect(a.current).toBe('new')
	})
})

describe('parseRawAppDiff — runnables', () => {
	it('emits per-runnable leaves for add/remove/modify', () => {
		const original = {
			runnables: { a: { path: 'u/x/a' }, b: { path: 'u/x/b' }, same: { path: 'u/x/same' } }
		}
		const current = {
			runnables: { a: { path: 'u/x/a2' }, c: { path: 'u/x/c' }, same: { path: 'u/x/same' } }
		}
		const entries = parseRawAppDiff(original, current)
		expect(byPath(entries, 'runnables/a')?.status).toBe('modified')
		expect(byPath(entries, 'runnables/a')?.lang).toBe('yaml')
		expect(byPath(entries, 'runnables/b')?.status).toBe('removed')
		expect(byPath(entries, 'runnables/c')?.status).toBe('added')
		// identical runnable omitted
		expect(byPath(entries, 'runnables/same')).toBeUndefined()
	})
})

describe('parseRawAppDiff — metadata', () => {
	it('collapses summary/data/policy/custom_path into one app.yaml leaf', () => {
		const entries = parseRawAppDiff(
			{ summary: 'old summary', custom_path: 'foo' },
			{ summary: 'new summary', custom_path: 'foo' }
		)
		const meta = byPath(entries, RAW_APP_METADATA_PATH)
		expect(meta?.status).toBe('modified')
		expect(meta?.lang).toBe('yaml')
		expect(meta?.original).toContain('old summary')
		expect(meta?.current).toContain('new summary')
	})

	it('omits app.yaml when no metadata field changed', () => {
		const entries = parseRawAppDiff(
			{ files: { 'a.txt': '1' }, summary: 's' },
			{ files: { 'a.txt': '2' }, summary: 's' }
		)
		expect(byPath(entries, RAW_APP_METADATA_PATH)).toBeUndefined()
		expect(entries).toHaveLength(1)
	})
})

describe('parseRawAppDiff — whole app added / removed', () => {
	it('marks everything added when the original side is absent', () => {
		const current = {
			files: { 'index.html': '<h1>hi</h1>' },
			runnables: { a: { path: 'u/x/a' } },
			summary: 'brand new'
		}
		const entries = parseRawAppDiff(undefined, current)
		expect(entries.every((e) => e.status === 'added')).toBe(true)
		expect(byPath(entries, 'index.html')?.original).toBeUndefined()
		expect(byPath(entries, RAW_APP_METADATA_PATH)?.status).toBe('added')
	})

	it('marks everything removed when the current side is absent', () => {
		const original = {
			files: { 'index.html': '<h1>hi</h1>' },
			runnables: { a: { path: 'u/x/a' } },
			summary: 'going away'
		}
		const entries = parseRawAppDiff(original, undefined)
		expect(entries.every((e) => e.status === 'removed')).toBe(true)
		expect(byPath(entries, 'index.html')?.current).toBeUndefined()
	})
})

describe('parseRawAppDiff — collisions', () => {
	it('disambiguates synthesized paths against real files', () => {
		const original = { files: { 'app.yaml': 'real-old' }, summary: 'sa' }
		const current = { files: { 'app.yaml': 'real-new' }, summary: 'sb' }
		const entries = parseRawAppDiff(original, current)
		// The real file keeps the natural path.
		const realFile = byPath(entries, 'app.yaml')!
		expect(realFile.original).toBe('real-old')
		// The metadata leaf is pushed to a non-colliding path.
		const meta = byPath(entries, 'app.yaml~2')!
		expect(meta.original).toContain('sa')
		expect(meta.current).toContain('sb')
		// No two entries share a path.
		const paths = entries.map((e) => e.path)
		expect(new Set(paths).size).toBe(paths.length)
	})
})

describe('parseRawAppDiff — input shapes', () => {
	// getItemValue returns the deployed app row: files/runnables/data live under
	// `value`; summary/policy/custom_path at the top level.
	it('reads files/runnables/data from the `value` wrapper (app-row shape)', () => {
		const original = {
			summary: 'classy',
			policy: { execution_mode: 'viewer' },
			value: { files: { 'index.html': '<h1>a</h1>' }, runnables: {}, data: {} }
		}
		const current = {
			summary: 'classy',
			policy: { execution_mode: 'viewer' },
			value: { files: { 'index.html': '<h1>b</h1>' }, runnables: {}, data: {} }
		}
		const entries = parseRawAppDiff(original, current)
		const file = byPath(entries, 'index.html')
		expect(file?.status).toBe('modified')
		expect(file?.original).toBe('<h1>a</h1>')
		expect(file?.current).toBe('<h1>b</h1>')
	})

	it('still handles the flat draft shape (files at top level)', () => {
		const entries = parseRawAppDiff({ files: { 'a.ts': 'x' } }, { files: { 'a.ts': 'y' } })
		expect(byPath(entries, 'a.ts')?.status).toBe('modified')
	})

	it('prefers `value` over a stray top-level files key', () => {
		const entries = parseRawAppDiff(
			{ files: { 'top.ts': 'ignored' }, value: { files: { 'real.ts': 'a' } } },
			{ files: { 'top.ts': 'ignored' }, value: { files: { 'real.ts': 'b' } } }
		)
		expect(byPath(entries, 'real.ts')?.status).toBe('modified')
		expect(byPath(entries, 'top.ts')).toBeUndefined()
	})
})

describe('rawAppDiffToItems', () => {
	const appPath = 'u/admin/classy_app'

	it('produces composite-pathed items with status-derived exists flags', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ value: { files: { '/App.tsx': 'a', '/gone.ts': 'x' }, runnables: { r: { p: 1 } } } },
			{ value: { files: { '/App.tsx': 'b', '/new.ts': 'y' }, runnables: {} } }
		)
		const byPath = (p: string) => items.find((i) => i.path === p)

		const app = byPath('u/admin/classy_app/App.tsx')!
		expect(app.kind).toBe('raw_app_file')
		expect(app.status).toBe('modified')
		expect(app.exists_in_source).toBe(true)
		expect(app.exists_in_fork).toBe(true)
		expect(app.appPath).toBe(appPath)
		expect((app as any).lang).toBe('typescript')

		const gone = byPath('u/admin/classy_app/gone.ts')!
		expect(gone.status).toBe('removed')
		expect(gone.exists_in_source).toBe(true)
		expect(gone.exists_in_fork).toBe(false)

		const added = byPath('u/admin/classy_app/new.ts')!
		expect(added.status).toBe('added')
		expect(added.exists_in_source).toBe(false)
		expect(added.exists_in_fork).toBe(true)

		// Runnables render as script/flow rows, not file leaves.
		const runnable = byPath('u/admin/classy_app/runnables/r')!
		expect(runnable.kind).toBe('script')
		expect(runnable.status).toBe('removed')
	})

	it('renders an inline-script runnable as a script row with code hoisted', () => {
		const mk = (code: string) => ({
			value: {
				files: {},
				runnables: {
					a: { name: 'a', type: 'inline', inlineScript: { content: code, language: 'bun' } }
				}
			}
		})
		const items = rawAppDiffToItems(appPath, mk('old code'), mk('new code'))
		const r = items.find((i) => i.path === `${appPath}/runnables/a`)
		expect(r?.kind).toBe('script')
		expect(r?.status).toBe('modified')
		// content + language hoisted to the top level for the script-style viewer
		const cur = (r as any).currentRaw
		expect(cur.content).toBe('new code')
		expect(cur.language).toBe('bun')
		expect(cur.inlineScript.content).toBeUndefined()
	})

	it('picks the flow kind for a runType:flow runnable', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ value: { files: {}, runnables: {} } },
			{ value: { files: {}, runnables: { f: { runType: 'flow', path: 'u/x/f' } } } }
		)
		const r = items.find((i) => i.path === `${appPath}/runnables/f`)
		expect(r?.kind).toBe('flow')
		expect(r?.status).toBe('added')
	})

	it('flags the metadata item and attaches whole-app YAML for the expand view', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ summary: 'old', value: { files: {} } },
			{ summary: 'new', value: { files: {} } }
		)
		const meta = items.find((i) => i.path === `${appPath}/${RAW_APP_METADATA_PATH}`) as any
		expect(meta.kind).toBe('raw_app_file')
		expect(meta.isMetadata).toBe(true)
		expect(meta.fullYamlOriginal).toContain('old')
		expect(meta.fullYamlCurrent).toContain('new')
	})

	it('keeps the metadata flag on the right item when a real file is named app.yaml', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ summary: 'old', value: { files: { 'app.yaml': 'real-old' } } },
			{ summary: 'new', value: { files: { 'app.yaml': 'real-new' } } }
		)
		// Real file keeps the natural path and is NOT the metadata item.
		const realFile = items.find((i) => i.path === `${appPath}/app.yaml`) as any
		expect(realFile.isMetadata).toBe(false)
		expect(realFile.fullYamlCurrent).toBeUndefined()
		// Synthesized metadata moved to app.yaml~2 but still carries the flag + YAML.
		const meta = items.find((i) => i.path === `${appPath}/app.yaml~2`) as any
		expect(meta.isMetadata).toBe(true)
		expect(meta.fullYamlCurrent).toContain('new')
	})

	it('keeps the metadata flag on the right item when a real file is named /app.yaml (leading slash)', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ summary: 'old', value: { files: { '/app.yaml': 'real-old' } } },
			{ summary: 'new', value: { files: { '/app.yaml': 'real-new' } } }
		)
		const realFile = items.find((i) => i.path === `${appPath}/app.yaml`) as any
		const meta = items.find((i) => i.path === `${appPath}/app.yaml~2`) as any
		expect(realFile.isMetadata).toBe(false)
		expect(meta.isMetadata).toBe(true)
		// distinct composite paths → distinct row keys
		expect(realFile.path).not.toBe(meta.path)
	})

	it('treats a file keyed /App.tsx on one side and App.tsx on the other as one item', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ value: { files: { '/App.tsx': 'old' } } },
			{ value: { files: { 'App.tsx': 'new' } } }
		)
		const files = items.filter((i) => i.path === `${appPath}/App.tsx`)
		expect(files).toHaveLength(1)
		expect(files[0].status).toBe('modified')
		expect((files[0] as any).original).toBe('old')
		expect((files[0] as any).current).toBe('new')
	})

	it('dedups a runnable leaf against a real file named runnables/<name>', () => {
		const items = rawAppDiffToItems(
			appPath,
			{ value: { files: { '/runnables/foo': 'old' }, runnables: { foo: { p: 1 } } } },
			{ value: { files: { '/runnables/foo': 'new' }, runnables: { foo: { p: 2 } } } }
		)
		const realFile = items.find((i) => i.kind === 'raw_app_file')!
		const runnable = items.find((i) => i.kind === 'script')!
		expect(realFile.path).toBe(`${appPath}/runnables/foo`)
		// Reserved away from the real file's composite path (slash-normalized).
		expect(runnable.path).toBe(`${appPath}/runnables/foo~2`)
		expect(runnable.path).not.toBe(realFile.path)
	})
})
