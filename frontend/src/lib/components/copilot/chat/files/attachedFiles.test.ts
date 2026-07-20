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
	isIgnoredPath: (p: string) =>
		p.split('/').some((s) => s.startsWith('.') || ['node_modules', 'dist', '.git'].includes(s)),
	queryReadPermission: vi.fn(async () => 'granted'),
	requestReadPermission: vi.fn(async () => 'granted')
}))

// buildLineIndex is real by default; a single test flips to 'manual' to control
// completion ordering and exercise the stale-index race guard.
type BuildResult = { lineIndex: number[]; lineCount: number }
const buildDeferreds: Array<{ file: Blob; resolve: (r: BuildResult) => void }> = []
let buildMode: 'real' | 'manual' = 'real'
vi.mock('./fileEngine', async (importOriginal) => {
	const actual = await importOriginal<typeof import('./fileEngine')>()
	return {
		...actual,
		buildLineIndex: (file: Blob) =>
			buildMode === 'real'
				? actual.buildLineIndex(file)
				: new Promise<BuildResult>((resolve) => buildDeferreds.push({ file, resolve }))
	}
})

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

	it('regrant of an empty folder keeps it linked and refreshing (not unlinked)', async () => {
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

		// Access re-granted, but the folder is currently empty — it must stay linked (ready
		// placeholder), not vanish when the locked placeholder is dropped.
		enumerateDirMock.mockResolvedValueOnce([])
		await s2.regrantLocked()
		expect(s2.folders).toEqual([{ name: 'proj', status: 'ready', files: [] }])
		expect(s2.lockedCount).toBe(0)

		// A file added afterward is picked up — the handle survived.
		enumerateDirMock.mockResolvedValueOnce([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		await s2.refreshFolders()
		await settle(s2)
		expect(s2.folders[0].files.map((f) => f.relPath)).toEqual(['proj/app.ts'])
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

	it('snapshots a folder via addFiles (paths), grouping it and persisting folder + relPath', async () => {
		const { putItem } = await import('./attachedFilesDB')
		// A persisted (non-transient) session writes through to IndexedDB immediately.
		const s = new AttachedFilesStore()
		await s.restore('s1', true)
		await s.addFiles([
			{ file: file('a.ts', 'x\n'), path: 'proj/a.ts' },
			{ file: file('b.ts', 'y\n'), path: 'proj/sub/b.ts' }
		])
		await settle(s)
		expect(s.folders.map((f) => f.name)).toEqual(['proj'])
		expect(s.folders[0].files.map((f) => f.relPath).sort()).toEqual(['proj/a.ts', 'proj/sub/b.ts'])
		expect(s.standalone).toEqual([])
		const rec = (putItem as ReturnType<typeof vi.fn>).mock.calls
			.map((c) => c[0])
			.find((r) => r.name === 'proj/a.ts')
		expect(rec).toMatchObject({ kind: 'snapshot', folder: 'proj', relPath: 'proj/a.ts' })
	})

	it('keeps same-basename files from different folder subdirs (dedup by path, not basename)', async () => {
		// Two distinct files with the same basename, size and lastModified, different subdirs.
		const res = await store.addFiles([
			{ file: file('index.ts', 'a\n', 5), path: 'proj/a/index.ts' },
			{ file: file('index.ts', 'a\n', 5), path: 'proj/b/index.ts' }
		])
		await settle(store)
		expect(res.added.sort()).toEqual(['proj/a/index.ts', 'proj/b/index.ts'])
		expect(store.folders[0].files.map((f) => f.relPath).sort()).toEqual([
			'proj/a/index.ts',
			'proj/b/index.ts'
		])
	})

	it('skips junk paths (node_modules/.git/dotfiles) inside a snapshotted folder', async () => {
		const res = await store.addFiles([
			{ file: file('a.ts', 'x\n'), path: 'proj/a.ts' },
			{ file: file('dep.js', 'z\n'), path: 'proj/node_modules/dep.js' },
			{ file: file('cfg', 'w\n'), path: 'proj/.git/config' }
		])
		await settle(store)
		expect(res.added).toEqual(['proj/a.ts'])
		expect(store.folders[0].files).toHaveLength(1)
	})

	it('keeps an explicitly attached standalone dotfile (filter is folder-only)', async () => {
		const res = await store.addFiles([file('.env', 'SECRET=1\n')])
		await settle(store)
		expect(res.added).toEqual(['.env'])
		expect(store.standalone.map((f) => f.name)).toEqual(['.env'])
	})

	it('restores a snapshot folder grouped from its persisted folder/relPath', async () => {
		const { getItemsForSession } = await import('./attachedFilesDB')
		;(getItemsForSession as ReturnType<typeof vi.fn>).mockResolvedValueOnce([
			{
				id: 's-a',
				sessionId: 's1',
				kind: 'snapshot',
				name: 'proj/a.ts',
				folder: 'proj',
				relPath: 'proj/a.ts',
				blob: file('a.ts', 'x\n'),
				addedAt: 0
			},
			{
				id: 's-b',
				sessionId: 's1',
				kind: 'snapshot',
				name: 'proj/b.ts',
				folder: 'proj',
				relPath: 'proj/b.ts',
				blob: file('b.ts', 'y\n'),
				addedAt: 0
			}
		])
		const s2 = new AttachedFilesStore()
		await s2.restore('s1', true)
		await settle(s2)
		expect(s2.folders.map((f) => f.name)).toEqual(['proj'])
		expect(s2.folders[0].files).toHaveLength(2)
		expect(s2.standalone).toEqual([])
	})

	it('imposes no file-count cap on a folder', async () => {
		enumerateDirMock.mockResolvedValue(
			Array.from({ length: 150 }, (_, i) => ({
				file: file(`f${i}.ts`, 'x\n'),
				path: `proj/f${i}.ts`
			}))
		)
		await store.addFolder(dir)
		await settle(store)
		expect(store.folders[0].files.length).toBe(150)
	})

	it('removeFolder deletes every snapshot record from storage (persisted session)', async () => {
		const { deleteItem } = await import('./attachedFilesDB')
		const s = new AttachedFilesStore()
		await s.restore('s1', true)
		await s.addFiles([
			{ file: file('a.ts', 'x\n'), path: 'proj/a.ts' },
			{ file: file('b.ts', 'y\n'), path: 'proj/sub/b.ts' }
		])
		await settle(s)
		const ids = s
			.list()
			.filter((f) => f.folder === 'proj')
			.map((f) => f.sourceId)
		expect(ids.length).toBe(2)
		;(deleteItem as ReturnType<typeof vi.fn>).mockClear()
		s.removeFolder('proj')
		expect(s.count).toBe(0)
		const deleted = (deleteItem as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0])
		for (const id of ids) expect(deleted).toContain(id)
	})

	it('a stale index completion does not corrupt a re-added same-named file', async () => {
		buildMode = 'manual'
		try {
			const A = file('a.txt', 'AAA\n')
			const B = file('a.txt', 'BBB\nBBB\nBBB\n')
			await store.addFiles([A]) // row 'a.txt' (file A) → buildLineIndex(A) pending
			store.removeFile('a.txt')
			await store.addFiles([B]) // new row 'a.txt' (file B) → buildLineIndex(B) pending

			// The old (stale) index for A resolves last — it must NOT touch the row now holding B.
			buildDeferreds.find((d) => d.file === A)!.resolve({ lineIndex: [0], lineCount: 99 })
			await Promise.resolve()
			expect(store.get('a.txt')?.status).toBe('indexing')
			expect(store.get('a.txt')?.lineCount).not.toBe(99)

			// B's own index applies normally.
			buildDeferreds.find((d) => d.file === B)!.resolve({ lineIndex: [0, 4, 8], lineCount: 3 })
			await Promise.resolve()
			expect(store.get('a.txt')?.status).toBe('ready')
			expect(store.get('a.txt')?.lineCount).toBe(3)
		} finally {
			buildMode = 'real'
			buildDeferreds.length = 0
		}
	})

	it('removeFolder deletes the live folder record from storage (persisted session)', async () => {
		const { deleteItem } = await import('./attachedFilesDB')
		const s = new AttachedFilesStore()
		await s.restore('s1', true)
		enumerateDirMock.mockResolvedValue([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		await s.addFolder(dir)
		await settle(s)
		const sourceId = s.list().find((f) => f.folder === 'proj')?.sourceId
		;(deleteItem as ReturnType<typeof vi.fn>).mockClear()
		s.removeFolder('proj')
		expect(s.count).toBe(0)
		expect((deleteItem as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0])).toContain(sourceId)
	})

	it('an emptied live folder stays visible and refreshes when files return', async () => {
		enumerateDirMock.mockResolvedValue([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		await store.addFolder(dir)
		await settle(store)
		expect(store.folders.map((f) => f.name)).toEqual(['proj'])

		// Folder emptied on disk → the last child is removed but the folder persists (placeholder).
		enumerateDirMock.mockResolvedValue([])
		await store.refreshFolders()
		await settle(store)
		expect(store.folders).toEqual([{ name: 'proj', status: 'ready', files: [] }])
		expect(store.get('proj/app.ts')).toBeUndefined()
		expect(store.readyFiles()).toEqual([]) // placeholder is never a tool target

		// A file added back on disk is picked up — the live source survived the empty state.
		enumerateDirMock.mockResolvedValue([{ file: file('new.ts', 'y\n'), path: 'proj/new.ts' }])
		await store.refreshFolders()
		await settle(store)
		expect(store.folders[0].files.map((f) => f.relPath)).toEqual(['proj/new.ts'])
	})

	it('an empty-folder placeholder does not collide with a same-named standalone file', async () => {
		enumerateDirMock.mockResolvedValue([]) // empty folder "proj" → creates a placeholder named "proj"
		await store.addFolder(dir)
		await settle(store)

		// A standalone file literally named "proj" must NOT be deduped by the placeholder.
		const res = await store.addFiles([file('proj', 'hello\n')])
		await settle(store)
		expect(res.added).toEqual(['proj'])
		expect(store.standalone.map((f) => f.name)).toEqual(['proj'])
		expect(store.folders.map((f) => f.name)).toEqual(['proj'])

		// Removing that standalone leaves the folder's placeholder intact.
		store.removeFile('proj')
		expect(store.standalone).toEqual([])
		expect(store.folders).toEqual([{ name: 'proj', status: 'ready', files: [] }])
	})

	it('links an initially empty live folder (kept visible, persisted, refreshes)', async () => {
		const { putItem } = await import('./attachedFilesDB')
		const s = new AttachedFilesStore()
		await s.restore('s1', true)
		enumerateDirMock.mockResolvedValue([]) // folder is empty at link time
		const res = await s.addFolder(dir)
		await settle(s)
		expect(res.added).toEqual([])
		expect(s.folders).toEqual([{ name: 'proj', status: 'ready', files: [] }])
		// persisted as a dir-handle so it survives a reload despite being empty
		const persisted = (putItem as ReturnType<typeof vi.fn>).mock.calls.map((c) => c[0])
		expect(persisted.some((r) => r.kind === 'dir-handle' && r.folder === 'proj')).toBe(true)

		// a file added later is picked up — the source existed from the start.
		enumerateDirMock.mockResolvedValue([{ file: file('app.ts', 'x\n'), path: 'proj/app.ts' }])
		await s.refreshFolders()
		await settle(s)
		expect(s.folders[0].files.map((f) => f.relPath)).toEqual(['proj/app.ts'])
	})

	it('registers a message file colliding with a session file under a new name', async () => {
		await store.addFiles([file('notes.md', 'session content\n')])
		await settle(store)

		const res = await store.addFiles([file('notes.md', 'message content\n', 2)], {
			messageScoped: true
		})
		await settle(store)

		// Not swallowed by the session row; renamed so both stay addressable. The
		// caller rewrites the message's reference to the returned name.
		expect(res.added).toEqual(['notes (2).md'])
		expect(store.get('notes.md')?.messageScoped).toBeFalsy()
		expect(store.get('notes (2).md')?.messageScoped).toBe(true)

		// A footer removal of the session file must not take the message row along.
		store.removeFile('notes.md')
		expect(store.get('notes (2).md')?.messageScoped).toBe(true)
		expect(store.standalone).toEqual([])
	})

	it('syncMessageScoped keeps a message file on its exact name despite a session clash', async () => {
		// Session asset loaded first (as on restore), then a past chat carrying a
		// same-named message attachment is opened: the rebuild must reclaim the
		// exact transcript reference, not suffix it, or get() would read the wrong
		// (session) content.
		await store.addFiles([file('notes.md', 'session content\n')])
		await settle(store)

		await store.syncMessageScoped([{ name: 'notes.md', content: 'message content\n' }])
		await settle(store)

		// The prompt reference `notes.md` resolves to the message content; the
		// session row is renamed aside but stays addressable under the suffix.
		expect(store.get('notes.md')?.messageScoped).toBe(true)
		expect(await (store.get('notes.md')!.file as Blob).text()).toBe('message content\n')
		expect(store.get('notes (2).md')?.messageScoped).toBeFalsy()
		expect(await (store.get('notes (2).md')!.file as Blob).text()).toBe('session content\n')
	})

	it('syncMessageScoped reconciles rows to the transcript references', async () => {
		await store.syncMessageScoped([
			{ name: 'a.md', content: 'aaa\n' },
			{ name: 'b.md', content: 'bbb\n' }
		])
		await settle(store)
		expect(store.messageAttached.map((f) => f.name).sort()).toEqual(['a.md', 'b.md'])

		// A message dropped from the transcript prunes its row; the survivor stays.
		await store.syncMessageScoped([{ name: 'a.md', content: 'aaa\n' }])
		await settle(store)
		expect(store.messageAttached.map((f) => f.name)).toEqual(['a.md'])

		await store.syncMessageScoped([])
		expect(store.messageAttached).toEqual([])
	})

	it('overlapping reconciliations commit only the latest set', async () => {
		// Rapid chat switching fires syncs without awaiting the previous one; a
		// stale pass finishing last must not leak its conversation's files.
		const stale = store.syncMessageScoped([{ name: 'old.md', content: 'OLD sentinel\n' }])
		const latest = store.syncMessageScoped([{ name: 'new.md', content: 'NEW sentinel\n' }])
		await Promise.all([stale, latest])
		await settle(store)
		expect(store.messageAttached.map((f) => f.name)).toEqual(['new.md'])
	})

	it('syncMessageScoped replaces a stale row under the same name', async () => {
		await store.syncMessageScoped([{ name: 'a.md', content: 'old\n' }])
		await settle(store)

		// The transcript's copy changed (edited message): the row must be replaced
		// under the SAME name, not suffixed away from the reference.
		await store.syncMessageScoped([{ name: 'a.md', content: 'new\n' }])
		await settle(store)
		expect(store.messageAttached.map((f) => f.name)).toEqual(['a.md'])
		expect(await (store.get('a.md')!.file as Blob).text()).toBe('new\n')
	})

	it('keeps message-scoped files tool-readable but out of the footer roster', async () => {
		await store.addFiles([file('notes.md', 'hello\n')], { messageScoped: true })
		await settle(store)

		// Hidden from the session bar, listed for the tools, readable by name.
		expect(store.standalone).toEqual([])
		expect(store.messageAttached.map((f) => f.name)).toEqual(['notes.md'])
		expect(store.get('notes.md')?.status).toBe('ready')

		// Identical re-registration (retry / sync rebuild) reuses the row.
		const retry = await store.addFiles([file('notes.md', 'hello\n', 2)], { messageScoped: true })
		expect(retry.added).toEqual(['notes.md'])
		expect(store.messageAttached.map((f) => f.name)).toEqual(['notes.md'])

		// A same-named file with DIFFERENT content is another message's attachment:
		// it registers under a suffixed name so the earlier message's reference
		// keeps resolving to its own content.
		const second = await store.addFiles([file('notes.md', 'other\n', 3)], { messageScoped: true })
		await settle(store)
		expect(second.added).toEqual(['notes (2).md'])
		expect(store.messageAttached.map((f) => f.name).sort()).toEqual(['notes (2).md', 'notes.md'])
	})
})
