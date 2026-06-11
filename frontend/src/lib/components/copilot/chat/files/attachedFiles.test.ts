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

	it('links a folder via a directory handle (enumerating it internally)', async () => {
		enumerateDirMock.mockResolvedValue([
			{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' },
			{ file: file('old.ts', 'y\n'), path: 'proj/old.ts' }
		])
		await store.addFolder(dir)
		await settle(store)
		expect(enumerateDirMock).toHaveBeenCalledWith(dir)
		expect(names(store)).toEqual(['proj/app.ts', 'proj/old.ts'])
		expect(store.get('proj/app.ts')?.folder).toBe('proj')
	})

	it('refreshFolders detects rename, add, edit, and delete', async () => {
		enumerateDirMock.mockResolvedValue([
			{ file: file('app.ts', 'x\n', 1), path: 'proj/app.ts' },
			{ file: file('old.ts', 'y\n', 1), path: 'proj/old.ts' }
		])
		await store.addFolder(dir)
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

	it('exposes folders and standalone as structured views', async () => {
		enumerateDirMock.mockResolvedValue([
			{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' },
			{ file: file('b.ts', 'y\n'), path: 'proj/sub/b.ts' }
		])
		await store.addFolder(dir)
		await store.addFiles([file('solo.txt', 'one\n')])
		await settle(store)

		expect(store.folders.map((f) => f.name)).toEqual(['proj'])
		expect(store.folders[0].status).toBe('ready')
		expect(store.folders[0].files.map((f) => f.relPath).sort()).toEqual([
			'proj/app.ts',
			'proj/sub/b.ts'
		])
		expect(store.standalone.map((f) => f.name)).toEqual(['solo.txt'])
		expect(store.lockedCount).toBe(0)
	})

	it('a locked folder surfaces as one folder with no files', async () => {
		const { getItemsForSession } = await import('./attachedFilesDB')
		;(getItemsForSession as ReturnType<typeof vi.fn>).mockResolvedValueOnce([
			{
				id: 'src1',
				sessionId: 's1',
				kind: 'dir-handle',
				name: 'proj',
				folder: 'proj',
				handle: dir,
				addedAt: 0
			}
		])
		const { queryReadPermission } = await import('./fsAccess')
		;(queryReadPermission as ReturnType<typeof vi.fn>).mockResolvedValueOnce('prompt')

		const s2 = new AttachedFilesStore()
		await s2.restore('s1', true)

		expect(s2.folders).toEqual([{ name: 'proj', status: 'locked', files: [] }])
		expect(s2.standalone).toEqual([])
		expect(s2.lockedCount).toBe(1)
	})

	it('re-picking a locked folder relinks it instead of silently no-oping', async () => {
		const { getItemsForSession } = await import('./attachedFilesDB')
		;(getItemsForSession as ReturnType<typeof vi.fn>).mockResolvedValueOnce([
			{
				id: 'src1',
				sessionId: 's1',
				kind: 'dir-handle',
				name: 'proj',
				folder: 'proj',
				handle: dir,
				addedAt: 0
			}
		])
		const { queryReadPermission } = await import('./fsAccess')
		;(queryReadPermission as ReturnType<typeof vi.fn>).mockResolvedValueOnce('prompt')
		const s2 = new AttachedFilesStore()
		await s2.restore('s1', true)
		expect(s2.folders[0]?.status).toBe('locked')

		enumerateDirMock.mockResolvedValue([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		const result = await s2.addFolder(dir)
		await settle(s2)
		expect(result.added).toEqual(['proj/app.ts'])
		expect(s2.folders).toHaveLength(1)
		expect(s2.folders[0].status).toBe('ready')
	})

	it('rejects linking a second folder with the same name (visible, not silent)', async () => {
		enumerateDirMock.mockResolvedValue([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		await store.addFolder(dir)
		await settle(store)
		const result = await store.addFolder(dir)
		expect(result.added).toEqual([])
		expect(result.rejected[0]?.reason).toMatch(/already linked/)
		expect(store.folders).toHaveLength(1)
	})

	it('regrant keeps the folder visible as unavailable when enumeration fails', async () => {
		const { getItemsForSession } = await import('./attachedFilesDB')
		;(getItemsForSession as ReturnType<typeof vi.fn>).mockResolvedValueOnce([
			{
				id: 'src1',
				sessionId: 's1',
				kind: 'dir-handle',
				name: 'proj',
				folder: 'proj',
				handle: dir,
				addedAt: 0
			}
		])
		const { queryReadPermission } = await import('./fsAccess')
		;(queryReadPermission as ReturnType<typeof vi.fn>).mockResolvedValueOnce('prompt')
		const s2 = new AttachedFilesStore()
		await s2.restore('s1', true)
		expect(s2.lockedCount).toBe(1)

		// Permission re-granted, but the directory is gone from disk.
		enumerateDirMock.mockRejectedValueOnce(new Error('directory removed'))
		await s2.regrantLocked()
		expect(s2.folders).toEqual([{ name: 'proj', status: 'unavailable', files: [] }])
	})

	it('removeFolder drops all of a folder’s files', async () => {
		enumerateDirMock.mockResolvedValue([
			{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' },
			{ file: file('b.ts', 'y\n'), path: 'proj/b.ts' }
		])
		await store.addFolder(dir)
		store.removeFolder('proj')
		expect(store.count).toBe(0)
	})
})
