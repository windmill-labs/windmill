import { describe, expect, it, vi } from 'vitest'

// '../shared' transitively imports monaco (CSS) which the node test env can't load.
// fileTools only needs createToolDef from it (called at module load), so stub it.
vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({ name, description })
}))

import { searchFilesTool } from './fileTools'
import type { AttachedFile, AttachedFilesStore } from './attachedFiles.svelte'

/** Minimal store stub: searchFilesTool's empty-ready path only reads count/readyFiles/list. */
function fakeStore(rows: Array<Partial<AttachedFile>>): AttachedFilesStore {
	const files = rows as AttachedFile[]
	return {
		get count() {
			return files.length
		},
		readyFiles: () => files.filter((f) => f.status === 'ready' && !f.isFolderRoot),
		list: () => files
	} as unknown as AttachedFilesStore
}

async function runSearch(store: AttachedFilesStore): Promise<string> {
	const res = await searchFilesTool.fn({
		args: { pattern: 'x' },
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
