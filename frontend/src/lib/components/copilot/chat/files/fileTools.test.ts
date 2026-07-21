import { describe, expect, it, vi } from 'vitest'

// '../shared' transitively imports monaco (CSS) which the node test env can't load.
// fileTools only needs createToolDef from it (called at module load), so stub it.
vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({ name, description })
}))

import { searchFilesTool } from './fileTools'
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
