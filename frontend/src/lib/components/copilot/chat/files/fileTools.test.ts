import { describe, expect, it, vi } from 'vitest'

// '../shared' transitively imports monaco (CSS) which the node test env can't load.
// fileTools only needs createToolDef from it (called at module load), so stub it.
vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({ name, description })
}))

import { imagesAtPath, resultImages, searchFilesTool } from './fileTools'
import type { AttachedFile, AttachedFilesStore } from './attachedFiles.svelte'

/** Minimal store stub mirroring the real lookup contract (id first, then
 * session-name, then message-name — the real ordering is pinned in
 * attachedFiles.test.ts). */
function fakeStore(rows: Array<Partial<AttachedFile>>): AttachedFilesStore {
	const files = rows as AttachedFile[]
	return {
		get count() {
			return files.length
		},
		readyFiles: () => files.filter((f) => f.status === 'ready' && !f.isFolderRoot),
		list: () => files,
		resolve: (ref: string) =>
			files.find((f) => f.id === ref) ??
			files.find((f) => f.name === ref && !f.isFolderRoot && !f.messageScoped) ??
			files.find((f) => f.name === ref && f.messageScoped)
	} as unknown as AttachedFilesStore
}

async function runSearch(
	store: AttachedFilesStore,
	args: Record<string, unknown> = { pattern: 'x' }
): Promise<string> {
	const res = await searchFilesTool.fn({
		args,
		helpers: { attachedFiles: store },
		toolId: 't',
		toolCallbacks: { setToolStatus: () => {} }
	} as any)
	return res as string
}

describe('search_files — attachments present but nothing readable', () => {
	it('reports no searchable text for an empty/binary-only linked folder (only ready placeholders)', async () => {
		// An empty/all-binary folder leaves a single `ready` placeholder row, filtered out of readyFiles().
		const msg = await runSearch(fakeStore([{ name: 'proj', status: 'ready', isFolderRoot: true }]))
		expect(msg).toMatch(/no searchable text files/i)
		expect(msg).not.toMatch(/indexed/i)
	})

	it('tells the user to restore access when a restored folder is locked', async () => {
		const msg = await runSearch(fakeStore([{ name: 'proj', status: 'locked', isFolderRoot: true }]))
		expect(msg).toMatch(/restore access/i)
	})

	it('tells the user to re-link when files are unavailable', async () => {
		const msg = await runSearch(fakeStore([{ name: 'gone.txt', status: 'unavailable' }]))
		expect(msg).toMatch(/re-link/i)
	})

	it('reports an image row by its status until it is ready', async () => {
		const gone = await runSearch(
			fakeStore([{ name: 'a.png', status: 'unavailable', image: true }]),
			{
				pattern: 'x',
				file: 'a.png'
			}
		)
		expect(gone).toMatch(/re-link/i)
		const ready = await runSearch(fakeStore([{ name: 'a.png', status: 'ready', image: true }]), {
			pattern: 'x',
			file: 'a.png'
		})
		expect(ready).toMatch(/is an image/i)
		expect(ready).not.toMatch(/show_image/)
	})

	it('still reports indexing while a file is genuinely indexing', async () => {
		const msg = await runSearch(fakeStore([{ name: 'a.txt', status: 'indexing' }]))
		expect(msg).toMatch(/still being indexed/i)
	})

	it('prefers the indexing message when an indexing file coexists with an empty folder', async () => {
		const msg = await runSearch(
			fakeStore([
				{ name: 'proj', status: 'ready', isFolderRoot: true },
				{ name: 'a.txt', status: 'indexing' }
			])
		)
		expect(msg).toMatch(/still being indexed/i)
	})
})

describe('roster — folder names are raw disk keys, sanitized at render', () => {
	it('strips control characters from locked/unavailable folder lines', async () => {
		const { buildAttachedFilesRoster } = await import('./fileTools')
		const store = {
			folders: [{ name: 'bad\nfolder', status: 'locked', files: [] }],
			standalone: [],
			messageAttached: []
		} as any
		const roster = buildAttachedFilesRoster(store)
		expect(roster).toContain('- bad folder (locked')
		expect(roster).not.toContain('bad\nfolder')
	})
})

// Display names may collide (same-named attachments on different messages). In
// this environment `new Worker` throws, so searchFilesInWorker takes its
// main-thread fallback — the search logic exercised is the same.
describe('search_files — colliding display names', () => {
	const colliding = () =>
		fakeStore([
			{
				name: 'notes.md',
				status: 'ready',
				messageScoped: true,
				id: 'fAAA',
				file: new Blob(['alpha target\n'])
			},
			{
				name: 'notes.md',
				status: 'ready',
				messageScoped: true,
				id: 'fBBB',
				file: new Blob(['bravo target\n'])
			}
		])

	it('labels unscoped hits with the id that resolves back to the matching row', async () => {
		const out = await runSearch(colliding(), { pattern: 'target' })
		expect(out).toContain('notes.md (file id: fAAA):1: alpha target')
		expect(out).toContain('notes.md (file id: fBBB):1: bravo target')
	})

	it('an id-scoped search hits only its own row', async () => {
		const out = await runSearch(colliding(), { pattern: 'target', file: 'fBBB' })
		expect(out).toContain('bravo target')
		expect(out).not.toContain('alpha target')
	})
})

describe('resultImages', () => {
	it('finds the image a result holds and each render_all item, by its path', () => {
		expect(resultImages({ png: 'AAA' })).toEqual([{ format: 'png', path: [] }])
		expect(resultImages({ jpeg: { content: 'AAA' } })).toEqual([{ format: 'jpeg', path: [] }])
		expect(
			resultImages({ render_all: [{ markdown: '# a' }, { png: 'AAA' }, { gif: 'BBB' }] })
		).toEqual([
			{ format: 'png', path: ['render_all', 1] },
			{ format: 'gif', path: ['render_all', 2] }
		])
		expect(resultImages({ s3: 'charts/a.png' })).toEqual([{ s3: { s3: 'charts/a.png' } }])
		expect(resultImages('s3://secondary/b.jpg')).toEqual([
			{ s3: { s3: 'b.jpg', storage: 'secondary' } }
		])
		expect(resultImages([{ s3: 'a.png' }, { s3: 'b.webp' }])).toHaveLength(2)
		expect(resultImages([{ s3: 'plot.png' }, { s3: 'data.csv' }])).toEqual([
			{ s3: { s3: 'plot.png' } }
		])
		expect(resultImages([{ s3: 'plot.png' }, 'other'])).toEqual([])
		expect(resultImages({ s3: 'data.parquet' })).toEqual([])
		expect(resultImages({ png: 42 })).toEqual([])
		expect(resultImages('png')).toEqual([])
	})
})

describe('imagesAtPath', () => {
	// A list of inline images, which resultImages alone does not open.
	const result = { charts: [{ png: 'AAA' }, { jpeg: { content: 'BBB' } }], files: ['s3://x/c.png'] }

	it('opens the image a path points at, by its object or its base64 field', () => {
		expect(imagesAtPath(result, ['charts', 1])).toEqual([{ format: 'jpeg', path: ['charts', 1] }])
		expect(imagesAtPath(result, ['charts', 0, 'png'])).toEqual([
			{ format: 'png', path: ['charts', 0] }
		])
		expect(imagesAtPath(result, ['files', 0])).toEqual([{ s3: { s3: 'c.png', storage: 'x' } }])
	})

	it('opens the format a path names when the object holds several', () => {
		expect(imagesAtPath({ png: 'AAA', jpeg: 'BBB' }, ['jpeg'])).toEqual([
			{ format: 'jpeg', path: [] }
		])
		expect(imagesAtPath({ png: 'AAA' }, ['jpeg'])).toEqual([])
	})

	it('finds nothing where the path leads off the result', () => {
		expect(imagesAtPath(result, ['charts', 5])).toEqual([])
		expect(imagesAtPath(result, ['charts', 0, 'png', 'deeper'])).toEqual([])
		expect(imagesAtPath('text', [0])).toEqual([])
	})
})
