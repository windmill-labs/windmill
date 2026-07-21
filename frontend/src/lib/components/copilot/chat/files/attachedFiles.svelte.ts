/**
 * Session-scoped store of files/folders the user has linked to the GLOBAL AI chat.
 *
 * Persistence model (survives reload, keyed by session in ./attachedFilesDB):
 *  - FILES are always stored as a full-byte Blob snapshot — same on every browser,
 *    no permission re-grant, never "locked".
 *  - FOLDERS link as a live File System Access directory handle where the API exists
 *    (one record, re-enumerated live on restore — folder files are read through the
 *    handle, not copied). Where it doesn't (Firefox/Safari), a dropped/picked folder's
 *    files are snapshotted individually (each carrying its `folder`/`relPath`) so they
 *    regroup into the same folder chip on restore.
 *
 * Storage is bounded by the real browser quota (writes that exceed it are caught and
 * the item simply isn't persisted — it stays usable for the session). Persistence is
 * gated on the session being persisted (non-transient); links in a transient session
 * are buffered and flushed on the first send.
 */
import { createLongHash } from '$lib/editorLangUtils'
import { buildLineIndex, isTextFile, type FileEntry } from './fileEngine'
import {
	putItem,
	deleteItem,
	getItemsForSession,
	ensurePersistentStorage,
	type PersistedAttachedItem
} from './attachedFilesDB'
import { sanitizeAttachmentName } from '../textFileUtils'
import { enumerateDir, isIgnoredPath, queryReadPermission, requestReadPermission } from './fsAccess'

export type AttachedFileStatus = 'indexing' | 'ready' | 'error' | 'locked' | 'unavailable'

export interface AttachedFile extends FileEntry {
	size: number
	status: AttachedFileStatus
	error?: string
	/** Top-level folder this file came from (first path segment), if part of a folder. */
	folder?: string
	/** Persisted source-record id. Folder children share the folder's record id. */
	sourceId: string
	/** Parent directory handle (folder children only) — used to re-grant / re-enumerate. */
	handle?: FileSystemDirectoryHandle
	/** Relative path within the folder (folder children only) — stable key for refresh diffing. */
	relPath?: string
	/**
	 * Internal: a single placeholder row standing in for a not-yet-expanded folder
	 * (locked/unavailable). Consumers should read `store.folders` instead of testing this.
	 */
	isFolderRoot?: boolean
	/** Attached to a chat message: readable via the file tools like any other row,
	 * but hidden from the session footer bar (its chip lives on the message). */
	messageScoped?: boolean
	/** Stable content-hash id (message-scoped rows only) — the reference the
	 * transcript and prompt carry. Names are display-only and may collide;
	 * lookups join on this. See attachedTextFileId. */
	id?: string
	/** Pre-suffix display name when uniquifying renamed the row (session rows).
	 * Used only by re-link dedupe — a re-link arrives under the original name
	 * and must match the row it was suffixed into. */
	sourceName?: string
}

/** A linked folder as a first-class object — consumers read this instead of re-grouping rows. */
export interface AttachedFolder {
	name: string
	/** Aggregate status (locked > unavailable > indexing > error > ready). */
	status: AttachedFileStatus
	/** Child files; empty while the folder is locked/unavailable after a reload. */
	files: AttachedFile[]
}

/** Aggregate a folder's rows (children + a possible placeholder) into one status. */
function folderStatus(rows: AttachedFile[]): AttachedFileStatus {
	for (const status of ['locked', 'unavailable', 'indexing', 'error'] as const) {
		if (rows.some((f) => f.status === status)) return status
	}
	return 'ready'
}

export interface AddFilesResult {
	added: string[]
	rejected: { name: string; reason: string }[]
}

/** A file to link: a raw File, or `{ file, path? }` (path = relative display name). */
export type FileToAttach = File | { file: File; path?: string }

const EMPTY = new Blob([])

export class AttachedFilesStore {
	files = $state<AttachedFile[]>([])

	/** Session context, set by the runtime; persistence writes are gated on `#persisted`. */
	sessionId: string | undefined = undefined
	#persisted = false
	/** Records buffered while the session is transient (flushed on first send). */
	#pending: PersistedAttachedItem[] = []

	list(): AttachedFile[] {
		return this.files
	}
	get(name: string): AttachedFile | undefined {
		// Name lookup. A bare name is the roster's namespace: session links are
		// advertised by filename and have no other handle, so they resolve first —
		// a same-named message attachment must not shadow them (it is addressed by
		// id). Message rows resolve by name only as the fallback, for transcripts
		// persisted before ids existed.
		// Folder-root placeholders may share a name with a real file — never resolve to one.
		return (
			this.files.find((f) => f.name === name && !f.isFolderRoot && !f.messageScoped) ??
			this.files.find((f) => f.name === name && f.messageScoped)
		)
	}
	/** Resolve a tool-supplied file reference: stable id first (how message
	 * attachments are addressed), then name (session links + legacy prompts).
	 * The composite label the roster and search hits print — `name (file id: x)`
	 * — resolves too: models echo references verbatim, so the printed form must
	 * be a valid one. Exact matches win over label interpretation: a row whose
	 * literal filename happens to look like a label stays addressable by it. */
	resolve(ref: string): AttachedFile | undefined {
		const exact = this.files.find((f) => f.id === ref) ?? this.get(ref)
		if (exact) return exact
		const label = ref.match(/^(.*) \(file id: ([^)]+)\)$/)
		if (label) {
			return this.files.find((f) => f.id === label[2]) ?? this.get(label[1])
		}
		return undefined
	}
	readyFiles(): AttachedFile[] {
		// Folder-root placeholders aren't real files — never expose them to the read/search tools.
		return this.files.filter((f) => f.status === 'ready' && !f.isFolderRoot)
	}
	get count(): number {
		return this.files.length
	}

	/** Linked folders, children grouped and status aggregated (placeholder rows hidden). */
	folders: AttachedFolder[] = $derived.by(() => {
		const byName = new Map<string, AttachedFile[]>()
		for (const f of this.files) {
			if (!f.folder) continue
			const rows = byName.get(f.folder)
			if (rows) rows.push(f)
			else byName.set(f.folder, [f])
		}
		return [...byName.entries()].map(([name, rows]) => ({
			name,
			status: folderStatus(rows),
			files: rows.filter((f) => !f.isFolderRoot)
		}))
	})

	/** Files linked on their own (not as part of a folder or a message). */
	standalone: AttachedFile[] = $derived.by(() =>
		this.files.filter((f) => !f.folder && !f.messageScoped)
	)

	/** Files attached to chat messages — tool-readable, not shown in the footer bar. */
	messageAttached: AttachedFile[] = $derived.by(() =>
		this.files.filter((f) => !f.folder && f.messageScoped)
	)

	/** Number of locked folders needing a re-grant. */
	get lockedCount(): number {
		return this.folders.filter((f) => f.status === 'locked').length
	}

	clear(): void {
		this.files = []
		this.#pending = []
	}

	removeFile(name: string): void {
		// Session rows only: message-scoped rows are managed by syncMessageScoped, and
		// a footer chip removal must never take a same-named message attachment with it.
		// Target the real file only — never a folder-root placeholder that happens to share
		// the name (those are managed via removeFolder), else removing a same-named standalone
		// file would also drop the folder's placeholder.
		const f = this.files.find((x) => x.name === name && !x.isFolderRoot && !x.messageScoped)
		if (!f) return
		this.files = this.files.filter((x) => x !== f)
		void this.#deleteRecord(f.sourceId)
	}

	/**
	 * Reconcile message-scoped rows to exactly `wanted` — the union of files the
	 * current transcript references, joined on the stable id. The transcript is
	 * their durable home: rows are rebuilt from it on chat load and pruned when a
	 * rollback or an edit/retry truncation drops the message that carried them.
	 * Synchronous — decisions compare ids, never content — so rapid chat
	 * switching cannot interleave two reconciliations.
	 */
	syncMessageScoped(wanted: { name: string; content: string; id: string }[]): void {
		const wantedIds = new Set(wanted.map((f) => f.id))
		for (const f of this.files.filter((x) => x.messageScoped)) {
			if (!f.id || !wantedIds.has(f.id)) {
				this.files = this.files.filter((x) => x !== f)
				void this.#deleteRecord(f.sourceId)
			}
		}
		this.registerMessageFiles(wanted)
	}

	/**
	 * Register a message's attachments as tool-readable rows. Identity is the
	 * content-hash id: a row already holding the id is reused (retry, sync
	 * rebuild), and distinct files register independently even under one display
	 * name. Rows are NOT persisted here — their durable home is the chat
	 * transcript (DisplayMessage.files), from which syncMessageScoped rebuilds
	 * them on load; a second copy in IndexedDB would only drift.
	 */
	registerMessageFiles(files: { name: string; content: string; id: string }[]): void {
		for (const f of files) {
			if (this.files.some((x) => x.messageScoped && x.id === f.id)) continue
			this.#pushIndexing({
				name: f.name,
				file: new File([f.content], f.name, { type: 'text/plain' }),
				sourceId: f.id,
				messageScoped: true,
				id: f.id
			})
		}
	}

	/** Remove every file linked as part of the given folder (and its persisted record). */
	removeFolder(folder: string): void {
		const ids = new Set(this.files.filter((f) => f.folder === folder).map((f) => f.sourceId))
		this.files = this.files.filter((f) => f.folder !== folder)
		for (const id of ids) void this.#deleteRecord(id)
	}

	// ---------------------------------------------------------------- linking

	/**
	 * Link individual files — always stored as a Blob snapshot. Items carrying a folder
	 * path (`folder/sub/file`, from a dropped/picked folder) are grouped into a folder and
	 * have their junk paths (node_modules/.git/dotfiles) skipped; a loose single file is
	 * kept as-is (so an explicitly attached `.env` isn't filtered out).
	 */
	async addFiles(input: FileList | FileToAttach[]): Promise<AddFilesResult> {
		const result: AddFilesResult = { added: [], rejected: [] }

		for (const item of Array.from(input as ArrayLike<FileToAttach>)) {
			const file = item instanceof File ? item : item.file
			// rawPath keys everything that must match the disk (folder split, junk
			// filter, relPath); the sanitized form is what names may be compared and
			// stored as — mixing the two is how dedupe once compared raw against
			// sanitized and missed.
			const rawPath =
				(item instanceof File ? '' : (item.path ?? '')) ||
				(file as File & { webkitRelativePath?: string }).webkitRelativePath ||
				file.name ||
				'file'
			const desired = sanitizeAttachmentName(rawPath)

			const folder = rawPath.includes('/') ? rawPath.split('/')[0] : undefined
			if (folder && isIgnoredPath(rawPath)) continue // skip junk inside folders

			if (this.#isDuplicate(desired, rawPath, file)) {
				continue // silent no-op on re-link
			}

			const reason = await this.#preflight(file)
			if (reason) {
				result.rejected.push({ name: desired, reason })
				continue
			}

			const { name, sourceName } = this.#claimName(rawPath)
			const relPath = folder ? rawPath : undefined
			const sourceId = createLongHash()

			this.#pushIndexing({ name, sourceName, file, folder, sourceId, relPath })
			result.added.push(name)
			void this.#persist({
				id: sourceId,
				sessionId: this.sessionId ?? '',
				kind: 'snapshot',
				name,
				folder,
				relPath,
				blob: file,
				size: file.size,
				lastModified: file.lastModified,
				addedAt: Date.now()
			})
		}

		return result
	}

	/**
	 * Link a folder via a live directory handle (File System Access path only).
	 * Enumerates the handle internally (junk-filtered, capped) — the same walk used
	 * on restore and refresh, so callers never pre-enumerate.
	 */
	async addFolder(dirHandle: FileSystemDirectoryHandle): Promise<AddFilesResult> {
		const result: AddFilesResult = { added: [], rejected: [] }
		const folder = dirHandle.name
		const existing = this.files.filter((f) => f.folder === folder)
		if (existing.length > 0) {
			const placeholder = existing.length === 1 ? existing.find((f) => f.isFolderRoot) : undefined
			if (placeholder) {
				// Re-picking a folder that sits locked/unavailable after a reload is a natural
				// recovery gesture — replace the stale link with the freshly-granted handle.
				this.files = this.files.filter((f) => f.sourceId !== placeholder.sourceId)
				void this.#deleteRecord(placeholder.sourceId)
			} else {
				// Same basename, possibly a different directory — surface it instead of a silent no-op.
				result.rejected.push({ name: folder, reason: 'A folder with this name is already linked' })
				return result
			}
		}

		const files = await enumerateDir(dirHandle)
		const sourceId = createLongHash()
		for (const { file, path } of files) {
			if (!(await this.#sniffText(file))) {
				result.rejected.push({ name: path, reason: 'Not a text file' })
				continue
			}
			const { name, sourceName } = this.#claimName(path)
			this.#pushIndexing({
				name,
				sourceName,
				file,
				folder,
				sourceId,
				handle: dirHandle,
				relPath: path
			})
			result.added.push(name)
		}
		// Keep the folder represented even when it links empty (or all-binary): a placeholder
		// carries the handle so the chip stays and refreshFolders picks up files added later.
		// Persist unconditionally so an empty-at-link folder also survives a reload.
		this.#ensureFolderRow(sourceId, folder, dirHandle)
		void this.#persist({
			id: sourceId,
			sessionId: this.sessionId ?? '',
			kind: 'dir-handle',
			name: folder,
			folder,
			handle: dirHandle,
			addedAt: Date.now()
		})
		return result
	}

	// ------------------------------------------------------------- persistence

	/** Set session context and load any persisted items for it (called on activation). */
	async restore(sessionId: string, persisted: boolean): Promise<void> {
		this.sessionId = sessionId
		this.#persisted = persisted
		this.files = []
		this.#pending = []

		const items = await getItemsForSession(sessionId)
		for (const item of items) {
			try {
				if (item.kind === 'snapshot') {
					if (!item.blob) {
						this.#pushPlaceholder(item, 'unavailable')
						continue
					}
					// Claimed on read: rows persisted before sanitization existed must
					// come back clean AND unique — two legacy names can sanitize to the
					// same string, and both must stay independently resolvable.
					this.#pushIndexing({
						...this.#claimName(item.name),
						file: item.blob,
						folder: item.folder,
						relPath: item.relPath,
						sourceId: item.id
					})
				} else {
					// dir-handle (folder)
					const handle = item.handle as FileSystemDirectoryHandle
					if ((await queryReadPermission(handle)) === 'granted') {
						await this.#expandFolder(handle, item.id)
					} else {
						this.#pushPlaceholder(item, 'locked', true)
					}
				}
			} catch {
				this.#pushPlaceholder(item, 'unavailable', item.kind === 'dir-handle')
			}
		}
	}

	/** Re-grant any locked folder handles. MUST be called within a user gesture (e.g. on send). */
	async regrantLocked(): Promise<void> {
		const sources = new Map<string, AttachedFile>()
		for (const f of this.files) {
			if (f.status === 'locked' && f.handle) sources.set(f.sourceId, f)
		}
		if (sources.size === 0) return

		// Kick off all permission requests within the gesture, then process. A rejected
		// request (requestReadPermission never rejects, but stay defensive) counts as denied.
		const decided = await Promise.all(
			[...sources.values()].map((f) =>
				requestReadPermission(f.handle!).then(
					(perm) => ({ f, perm }),
					() => ({ f, perm: 'denied' as PermissionState })
				)
			)
		)
		for (const { f, perm } of decided) {
			if (perm !== 'granted') continue
			try {
				await this.#expandFolder(f.handle as FileSystemDirectoryHandle, f.sourceId)
				// Children are in — drop the locked placeholder row, then restore a ready
				// placeholder if the folder came back empty/all-binary (else dropping the only
				// handle-bearing row would unlink the folder and stop it ever refreshing).
				this.files = this.files.filter((x) => !(x.sourceId === f.sourceId && x.isFolderRoot))
				this.#ensureFolderRow(f.sourceId, f.folder ?? f.name, f.handle as FileSystemDirectoryHandle)
			} catch {
				// Enumeration failed (folder moved/deleted on disk): drop any partially-added
				// children and keep the placeholder so the chip shows "unavailable".
				this.files = this.files.filter((x) => x.sourceId !== f.sourceId || x.isFolderRoot)
				this.#patchSource(f.sourceId, { status: 'unavailable' })
			}
		}
	}

	/** Flush buffered links once the session becomes persistent (first send). */
	async flushPending(): Promise<void> {
		this.#persisted = true
		if (!this.sessionId) return
		const pending = this.#pending
		this.#pending = []
		if (pending.length === 0) return
		void ensurePersistentStorage()
		for (const item of pending) {
			try {
				await putItem({ ...item, sessionId: this.sessionId })
			} catch (e) {
				console.error('Could not persist linked file', e)
			}
		}
	}

	async #persist(item: PersistedAttachedItem): Promise<void> {
		if (this.#persisted && this.sessionId) {
			void ensurePersistentStorage()
			try {
				// A QuotaExceededError just means it won't survive a reload — the item
				// stays usable for this session. Swallow + log rather than fail the link.
				await putItem({ ...item, sessionId: this.sessionId })
			} catch (e) {
				console.error('Could not persist linked file (kept for this session)', e)
			}
		} else {
			this.#pending.push(item)
		}
	}

	async #deleteRecord(sourceId: string): Promise<void> {
		this.#pending = this.#pending.filter((p) => p.id !== sourceId)
		if (this.#persisted) {
			try {
				await deleteItem(sourceId)
			} catch {
				/* ignore */
			}
		}
	}

	// ------------------------------------------------------------- internals

	/** Identical session-file re-link (same name, or same File identity) → silent no-op.
	 * Session rows only — message rows dedupe by their content-hash id in
	 * registerMessageFiles, and a same-named row of the other scope is a
	 * different file, not a duplicate. */
	#isDuplicate(name: string, rawPath: string, file: File): boolean {
		// "Duplicate" means the SAME file re-linked — identical stats plus a
		// matching name, pre-suffix name, or raw path. Never a bare display-name
		// match: two distinct raw names can sanitize to one display name and both
		// must survive (#claimName suffixes the second). A same-named file with
		// different stats is a different (or edited) file and links as its own row.
		return this.files.some(
			(f) =>
				// Folder-root placeholders aren't real files — they must not block attaching a
				// standalone file that happens to share the folder's name.
				!f.isFolderRoot &&
				!f.messageScoped &&
				f.size === file.size &&
				f.file instanceof File &&
				f.file.lastModified === file.lastModified &&
				// Sanitized-name compare (stored names are sanitized); sourceName catches
				// a re-link of a row that uniquifying suffixed; relPath keys folder
				// children on the RAW on-disk path, NOT the basename — two distinct files
				// sharing a basename under different subdirs must not dedupe.
				(f.name === name || f.sourceName === name || f.relPath === rawPath)
		)
	}

	/** Returns a rejection reason, or undefined if the file may be linked. */
	async #preflight(file: File): Promise<string | undefined> {
		if (!(await this.#sniffText(file))) return 'Not a text file'
		return undefined
	}

	async #sniffText(file: Blob): Promise<boolean> {
		try {
			return await isTextFile(file)
		} catch {
			return false
		}
	}

	#pushIndexing(p: {
		name: string
		sourceName?: string
		file: File | Blob
		folder?: string
		sourceId: string
		handle?: FileSystemDirectoryHandle
		relPath?: string
		messageScoped?: boolean
		id?: string
	}): void {
		this.files = [
			...this.files,
			{
				name: p.name,
				sourceName: p.sourceName,
				file: p.file,
				size: p.file.size,
				lineIndex: [],
				lineCount: 0,
				status: 'indexing',
				folder: p.folder,
				sourceId: p.sourceId,
				handle: p.handle,
				relPath: p.relPath,
				messageScoped: p.messageScoped,
				id: p.id
			}
		]
		void this.#indexFile(p.name, p.file)
	}

	#pushPlaceholder(
		item: PersistedAttachedItem,
		status: AttachedFileStatus,
		isFolderRoot = false
	): void {
		// File placeholders claim like any row (a legacy name may collide once
		// sanitized); folder-root placeholders keep the folder key's name and stay
		// outside name uniqueness — they may legitimately share a name with a file.
		const { name, sourceName } = isFolderRoot
			? { name: sanitizeAttachmentName(item.name), sourceName: undefined }
			: this.#claimName(item.name)
		this.files = [
			...this.files,
			{
				name,
				sourceName,
				file: EMPTY,
				size: item.size ?? 0,
				lineIndex: [],
				lineCount: 0,
				status,
				folder: item.folder,
				sourceId: item.id,
				handle: item.handle as FileSystemDirectoryHandle | undefined,
				isFolderRoot
			}
		]
	}

	async #expandFolder(dirHandle: FileSystemDirectoryHandle, sourceId: string): Promise<void> {
		const folder = dirHandle.name
		const children = await enumerateDir(dirHandle)
		for (const { file, path } of children) {
			if (!(await this.#sniffText(file))) continue
			const { name, sourceName } = this.#claimName(path)
			this.#pushIndexing({
				name,
				sourceName,
				file,
				folder,
				sourceId,
				handle: dirHandle,
				relPath: path
			})
		}
		this.#ensureFolderRow(sourceId, folder, dirHandle)
	}

	/**
	 * Re-enumerate granted folder handles to reflect on-disk changes since they were
	 * linked/last refreshed: added/removed/renamed files and content edits. Called on
	 * each send so the AI sees the folder's current state. Unchanged files are left as-is
	 * (diffed by relative path + lastModified); only changed files are re-indexed.
	 */
	async refreshFolders(): Promise<void> {
		const sources = new Map<string, { handle: FileSystemDirectoryHandle; folder: string }>()
		for (const f of this.files) {
			// Include folder-root placeholders (an emptied folder keeps only its placeholder),
			// else the source is lost and the folder never re-enumerates again.
			if (f.folder && f.handle) {
				sources.set(f.sourceId, { handle: f.handle, folder: f.folder })
			}
		}
		for (const [sourceId, { handle, folder }] of sources) {
			try {
				if ((await queryReadPermission(handle)) !== 'granted') continue
				const children = await enumerateDir(handle)
				await this.#reconcileFolder(sourceId, folder, handle, children)
			} catch {
				this.#patchSource(sourceId, { status: 'unavailable' })
			}
		}
	}

	async #reconcileFolder(
		sourceId: string,
		folder: string,
		handle: FileSystemDirectoryHandle,
		children: { file: File; path: string }[]
	): Promise<void> {
		const existing = new Map<string, AttachedFile>()
		for (const f of this.files) if (f.sourceId === sourceId && f.relPath) existing.set(f.relPath, f)
		const seen = new Set<string>()

		for (const { file, path } of children) {
			seen.add(path)
			const cur = existing.get(path)
			if (!cur) {
				// newly added on disk
				if (!(await this.#sniffText(file))) continue
				const { name, sourceName } = this.#claimName(path)
				this.#pushIndexing({ name, sourceName, file, folder, sourceId, handle, relPath: path })
			} else {
				const curMod = cur.file instanceof File ? cur.file.lastModified : undefined
				if (file.size !== cur.size || file.lastModified !== curMod) {
					// content changed → re-read + re-index. Patch by row identity — a
					// message attachment may share the display name.
					this.files = this.files.map((f) =>
						f === cur ? { ...f, file, size: file.size, status: 'indexing' } : f
					)
					void this.#indexFile(cur.name, file)
				}
			}
		}
		// removed/renamed-away on disk → drop from memory (by row identity — a
		// message attachment may share the display name and must survive)
		const removed = new Set([...existing.values()].filter((f) => f.relPath && !seen.has(f.relPath)))
		if (removed.size > 0) {
			this.files = this.files.filter((f) => !removed.has(f))
		}
		this.#ensureFolderRow(sourceId, folder, handle)
	}

	/**
	 * Keep a linked folder represented even with no readable children: leave one
	 * handle-carrying placeholder row so the chip stays visible AND `refreshFolders`
	 * keeps the live source (without it, an emptied folder vanishes and never
	 * re-enumerates). Drop the placeholder as soon as real children exist again.
	 */
	#ensureFolderRow(sourceId: string, folder: string, handle: FileSystemDirectoryHandle): void {
		const hasChild = this.files.some((f) => f.sourceId === sourceId && !f.isFolderRoot)
		const hasPlaceholder = this.files.some((f) => f.sourceId === sourceId && f.isFolderRoot)
		if (!hasChild && !hasPlaceholder) {
			this.files = [
				...this.files,
				{
					name: folder,
					file: EMPTY,
					size: 0,
					lineIndex: [],
					lineCount: 0,
					status: 'ready',
					folder,
					sourceId,
					handle,
					isFolderRoot: true
				}
			]
		} else if (hasChild && hasPlaceholder) {
			this.files = this.files.filter((f) => !(f.sourceId === sourceId && f.isFolderRoot))
		}
	}

	async #indexFile(name: string, file: File | Blob): Promise<void> {
		try {
			const { lineIndex, lineCount } = await buildLineIndex(file)
			this.#patchFile(name, file, { lineIndex, lineCount, status: 'ready' })
		} catch (e) {
			this.#patchFile(name, file, {
				status: 'error',
				error: e instanceof Error ? e.message : String(e)
			})
		}
	}

	/**
	 * Patch the row for `name` ONLY while it still holds the exact `file` we indexed.
	 * `buildLineIndex` is async and unawaited; between its start and finish the row's
	 * file can be swapped (remove + re-add a same-named file, or a folder refresh
	 * re-indexing an edited file). Without the identity check a stale completion would
	 * stamp the wrong lineIndex/lineCount on the new file, and read_file would then slice
	 * the new Blob with the old offsets.
	 */
	#patchFile(name: string, file: File | Blob, changes: Partial<AttachedFile>): void {
		this.files = this.files.map((f) =>
			f.name === name && f.file === file ? { ...f, ...changes } : f
		)
	}
	#patchSource(sourceId: string, changes: Partial<AttachedFile>): void {
		this.files = this.files.map((f) => (f.sourceId === sourceId ? { ...f, ...changes } : f))
	}

	/** The only way a session row gets its display name: sanitize (control
	 * characters must never reach model-facing text) then uniquify among session
	 * rows. Every creation path — attach, folder expansion, refresh, persisted
	 * restore — goes through here so none can skip a rule. Message rows stay out
	 * by design (id-addressed, names may collide); `relPath` and `folder` stay
	 * raw by design (on-disk keys). When uniquifying renamed the row,
	 * `sourceName` records the pre-suffix name so re-link dedupe can still
	 * recognize the same source file. */
	#claimName(raw: string): { name: string; sourceName?: string } {
		const base = sanitizeAttachmentName(raw)
		const name = this.#uniqueName(base)
		return name === base ? { name } : { name, sourceName: base }
	}

	#uniqueName(original: string): string {
		// Uniqueness is only among session rows: folder-root placeholders aren't real
		// files, and message rows are addressed by id — their display names neither
		// block a session link nor need protecting from one.
		const taken = (n: string) =>
			this.files.some((f) => f.name === n && !f.isFolderRoot && !f.messageScoped)
		if (!taken(original)) return original
		const dot = original.lastIndexOf('.')
		const base = dot > 0 ? original.slice(0, dot) : original
		const ext = dot > 0 ? original.slice(dot) : ''
		let n = 2
		let candidate = `${base} (${n})${ext}`
		while (taken(candidate)) {
			n++
			candidate = `${base} (${n})${ext}`
		}
		return candidate
	}
}
