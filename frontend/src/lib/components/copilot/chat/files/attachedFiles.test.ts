import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock persistence + File System Access so we exercise the in-memory store logic.
vi.mock('./attachedFilesDB', () => ({
	putItem: vi.fn(async () => {}),
	deleteItem: vi.fn(async () => {}),
	getItemsForSession: vi.fn(async () => []),
	ensurePersistentStorage: vi.fn(async () => {})
}))

const enumerateDirMock = vi.fn<(h: unknown) => Promise<{ file: File; path: string }[]>>()
vi.mock('./fsAccess', () => ({
	enumerateDir: (h: unknown) => enumerateDirMock(h),
	queryReadPermission: vi.fn(async () => 'granted'),
	requestReadPermission: vi.fn(async () => 'granted')
}))

import { AttachedFilesStore } from './attachedFiles.svelte'

function file(name: string, content: string, lastModified = 1): File {
	return new File([content], name, { type: 'text/plain', lastModified })
}

const dir = { kind: 'directory', name: 'proj' } as unknown as FileSystemDirectoryHandle

async function settle(store: AttachedFilesStore) {
	for (let i = 0; i < 100 && store.list().some((f) => f.status === 'indexing'); i++) {
		await new Promise((r) => setTimeout(r, 2))
	}
}

const names = (store: AttachedFilesStore) =>
	store
		.list()
		.map((f) => f.name)
		.sort()

describe('AttachedFilesStore', () => {
	let store: AttachedFilesStore
	beforeEach(async () => {
		store = new AttachedFilesStore()
		await store.restore('s1', false)
	})

	it('links and indexes individual files as snapshots', async () => {
		await store.addFiles([file('a.txt', 'one\ntwo\n')])
		await settle(store)
		const f = store.get('a.txt')
		expect(f?.status).toBe('ready')
		expect(f?.lineCount).toBe(2)
		expect(f?.handle).toBeUndefined() // files never carry a handle
	})

	it('removes a file', async () => {
		await store.addFiles([file('a.txt', 'x')])
		store.removeFile('a.txt')
		expect(store.count).toBe(0)
	})

	it('links a folder via a directory handle', async () => {
		await store.addFolder(dir, [
			{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' },
			{ file: file('old.ts', 'y\n'), path: 'proj/old.ts' }
		])
		await settle(store)
		expect(names(store)).toEqual(['proj/app.ts', 'proj/old.ts'])
		expect(store.get('proj/app.ts')?.folder).toBe('proj')
	})

	it('refreshFolders detects rename, add, edit, and delete', async () => {
		await store.addFolder(dir, [
			{ file: file('app.ts', 'x\n', 1), path: 'proj/app.ts' },
			{ file: file('old.ts', 'y\n', 1), path: 'proj/old.ts' }
		])
		await settle(store)

		// On disk: app.ts edited (mtime bumped), old.ts renamed → new.ts, readme.md added.
		enumerateDirMock.mockResolvedValue([
			{ file: file('app.ts', 'x\nedited\n', 2), path: 'proj/app.ts' },
			{ file: file('new.ts', 'y\n', 1), path: 'proj/new.ts' },
			{ file: file('readme.md', '# hi\n', 1), path: 'proj/readme.md' }
		])
		await store.refreshFolders()
		await settle(store)

		// old.ts dropped (renamed away); new.ts + readme.md added; app.ts kept.
		expect(names(store)).toEqual(['proj/app.ts', 'proj/new.ts', 'proj/readme.md'])
		// edited file re-indexed to its new content (2 lines).
		expect(store.get('proj/app.ts')?.status).toBe('ready')
		expect(store.get('proj/app.ts')?.lineCount).toBe(2)
	})

	it('removeFolder drops all of a folder’s files', async () => {
		await store.addFolder(dir, [
			{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' },
			{ file: file('b.ts', 'y\n'), path: 'proj/b.ts' }
		])
		store.removeFolder('proj')
		expect(store.count).toBe(0)
	})
})
