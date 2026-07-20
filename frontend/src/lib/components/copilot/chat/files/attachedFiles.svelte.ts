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
		// Message attachments take lookup precedence: a prompt reference names the
		// file attached to the message, not a same-named session link (sends rename
		// collisions away; duplicates can only arrive via legacy/load edges).
		// Folder-root placeholders may share a name with a real file — never resolve to one.
		return (
			this.files.find((f) => f.name === name && f.messageScoped) ??
			this.files.find((f) => f.name === name && !f.isFolderRoot)
		)
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

	#removeMessageScopedRow(name: string): void {
		const f = this.files.find((x) => x.messageScoped && x.name === name)
		if (!f) return
		this.files = this.files.filter((x) => x !== f)
		void this.#deleteRecord(f.sourceId)
	}

	/** Rename any session row currently holding `name` to a suffixed name, freeing
	 * it for a message-scoped rebuild that must land on its exact (immutable)
	 * transcript reference. In-memory only: the rename is deterministic and
	 * re-applied on every load (session items load, then syncMessageScoped runs),
	 * so it stays consistent without a persisted-record update. */
	#freeNameForMessageRow(name: string, wantedNames: ReadonlySet<string>): void {
		const clash = this.files.find((f) => f.name === name && !f.messageScoped && !f.isFolderRoot)
		if (!clash) return
		// Suffix the session row clear of the WHOLE wanted set, not just current
		// rows: renaming it onto another wanted name (e.g. freeing `notes.md` onto
		// `notes (2).md` when both are transcript references) would push that second
		// message row to a further suffix and orphan its persisted reference.
		const renamed = this.#uniqueName(name, wantedNames)
		this.files = this.files.map((f) => (f === clash ? { ...f, name: renamed } : f))
		// #indexFile (started at restore under the old name) stamps the row via
		// #patchFile(oldName, file) — after this rename that no longer matches, so an
		// in-flight index would leave the row stuck 'indexing'. Re-target it to the
		// new name; the stale completion then no-ops (its name is gone).
		if (clash.status === 'indexing') void this.#indexFile(renamed, clash.file)
	}

	#syncSeq = 0
	#syncChain: Promise<void> = Promise.resolve()

	/**
	 * Reconcile message-scoped rows to exactly `wanted` — the union of files the
	 * current transcript references. The transcript is their durable home: rows are
	 * rebuilt from it on chat load and pruned when a rollback or an edit/retry
	 * truncation drops the message that carried them.
	 *
	 * Serialized with supersession: the body awaits mid-mutation, and rapid chat
	 * switching fires overlapping reconciliations — interleaved, a stale pass
	 * could resume after a newer load and append another conversation's files.
	 * Passes run one at a time and a superseded pass no-ops, so only the latest
	 * transcript's set ever commits.
	 */
	syncMessageScoped(wanted: { name: string; content: string }[]): Promise<void> {
		const seq = ++this.#syncSeq
		const run = async () => {
			if (seq !== this.#syncSeq) return
			const wantedByName = new Map(wanted.map((f) => [f.name, f]))
			for (const f of this.files.filter((x) => x.messageScoped)) {
				const w = wantedByName.get(f.name)
				// Also drop a row whose content diverged from the transcript's copy —
				// keeping it would make the re-registration below suffix instead of
				// landing back on the referenced name.
				if (!w || (await (f.file as Blob).text()) !== w.content) {
					this.#removeMessageScopedRow(f.name)
				}
			}
			if (wanted.length > 0) {
				// A transcript reference name is immutable (baked into the persisted
				// prompt). Session rows load first (on restore), so one holding a
				// wanted name would push the rebuilt message row to a "(2)" suffix and
				// orphan the reference — get() would then resolve the prompt's name to
				// the session asset and read the wrong content. Rename the session row
				// aside so the message row reclaims its exact name; the session roster
				// is regenerated live each send, so it stays addressable under the suffix.
				const wantedNames = new Set(wanted.map((f) => f.name))
				for (const name of wantedNames) this.#freeNameForMessageRow(name, wantedNames)
				await this.addFiles(
					wanted.map((f) => new File([f.content], f.name, { type: 'text/plain' })),
					{ messageScoped: true }
				)
			}
		}
		// Run after the predecessor regardless of its outcome; keep the stored
		// chain rejection-free so one failed pass can't wedge every later one.
		const pass = this.#syncChain.then(run, run)
		this.#syncChain = pass.catch(() => {})
		return pass
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
	async addFiles(
		input: FileList | FileToAttach[],
		opts?: { messageScoped?: boolean }
	): Promise<AddFilesResult> {
		const result: AddFilesResult = { added: [], rejected: [] }
		const messageScoped = opts?.messageScoped ?? false

		for (const item of Array.from(input as ArrayLike<FileToAttach>)) {
			const file = item instanceof File ? item : item.file
			const desired =
				(item instanceof File ? '' : (item.path ?? '')) ||
				(file as File & { webkitRelativePath?: string }).webkitRelativePath ||
				file.name ||
				'file'

			const folder = desired.includes('/') ? desired.split('/')[0] : undefined
			if (folder && isIgnoredPath(desired)) continue // skip junk inside folders

			// A message references its files by name. An identical re-registration
			// (retry, sync rebuild) reuses the existing row; a same-name file with
			// DIFFERENT content is another message's attachment and must register
			// under a suffixed name — replacing would silently retarget the earlier
			// message's reference at the new content. (An edited message never
			// collides with its own old row: the edit truncation syncs it away
			// before the resend registers.) Compared by content, not #isDuplicate:
			// registration recreates the File each send, so lastModified would
			// misread a changed file as a mere re-link.
			if (messageScoped) {
				const existing = this.files.find((f) => f.name === desired && f.messageScoped)
				if (existing) {
					const sameContent =
						existing.size === file.size &&
						(await (existing.file as Blob).text()) === (await file.text())
					if (sameContent) {
						// Still reported: the caller maps prompt references off `added`.
						result.added.push(desired)
						continue
					}
				}
			} else if (this.#isDuplicate(desired, file)) {
				continue // silent no-op on re-link
			}

			const reason = await this.#preflight(file)
			if (reason) {
				result.rejected.push({ name: desired, reason })
				continue
			}

			const name = this.#uniqueName(desired)
			const relPath = folder ? desired : undefined
			const sourceId = createLongHash()

			this.#pushIndexing({ name, file, folder, sourceId, relPath, messageScoped })
			result.added.push(name)
			// Message rows are NOT persisted here: their durable home is the chat
			// transcript (DisplayMessage.files), from which syncMessageScoped rebuilds
			// them on load — a second copy in IndexedDB would only drift.
			if (!messageScoped) {
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
			const name = this.#uniqueName(path)
			this.#pushIndexing({ name, file, folder, sourceId, handle: dirHandle, relPath: path })
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
					this.#pushIndexing({
						name: item.name,
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
	 * Session rows only — message rows dedupe by content in addFiles, and a row of
	 * one scope must never swallow the other scope's same-named file (the model
	 * would then read the wrong content; collisions suffix via #uniqueName). */
	#isDuplicate(desired: string, file: File): boolean {
		return this.files.some(
			(f) =>
				// Folder-root placeholders aren't real files — they must not block attaching a
				// standalone file that happens to share the folder's name.
				!f.isFolderRoot &&
				!f.messageScoped &&
				(f.name === desired ||
					// Identical re-drop at the SAME relative path (its row name may have been
					// auto-suffixed). Keyed on the path, NOT the basename — otherwise two distinct
					// files sharing a basename under different folder subdirs (proj/a/index.ts vs
					// proj/b/index.ts) would be wrongly deduped and silently dropped.
					((f.relPath ?? f.name) === desired &&
						f.size === file.size &&
						f.file instanceof File &&
						f.file.lastModified === file.lastModified))
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
		file: File | Blob
		folder?: string
		sourceId: string
		handle?: FileSystemDirectoryHandle
		relPath?: string
		messageScoped?: boolean
	}): void {
		this.files = [
			...this.files,
			{
				name: p.name,
				file: p.file,
				size: p.file.size,
				lineIndex: [],
				lineCount: 0,
				status: 'indexing',
				folder: p.folder,
				sourceId: p.sourceId,
				handle: p.handle,
				relPath: p.relPath,
				messageScoped: p.messageScoped
			}
		]
		void this.#indexFile(p.name, p.file)
	}

	#pushPlaceholder(
		item: PersistedAttachedItem,
		status: AttachedFileStatus,
		isFolderRoot = false
	): void {
		this.files = [
			...this.files,
			{
				name: item.name,
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
			const name = this.#uniqueName(path)
			this.#pushIndexing({ name, file, folder, sourceId, handle: dirHandle, relPath: path })
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
				const name = this.#uniqueName(path)
				this.#pushIndexing({ name, file, folder, sourceId, handle, relPath: path })
			} else {
				const curMod = cur.file instanceof File ? cur.file.lastModified : undefined
				if (file.size !== cur.size || file.lastModified !== curMod) {
					// content changed → re-read + re-index
					this.#patch(cur.name, { file, size: file.size, status: 'indexing' })
					void this.#indexFile(cur.name, file)
				}
			}
		}
		// removed/renamed-away on disk → drop from memory
		const removed = [...existing.values()].filter((f) => f.relPath && !seen.has(f.relPath))
		if (removed.length > 0) {
			const names = new Set(removed.map((f) => f.name))
			this.files = this.files.filter((f) => !names.has(f.name))
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

	#patch(name: string, changes: Partial<AttachedFile>): void {
		this.files = this.files.map((f) => (f.name === name ? { ...f, ...changes } : f))
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

	#uniqueName(original: string, reserved?: ReadonlySet<string>): string {
		// Uniqueness is only among real files — folder-root placeholders may share a name
		// with a standalone file and must not push it to a "(2)" suffix. `reserved`
		// blocks extra names the caller must keep free (e.g. other transcript references).
		const taken = (n: string) =>
			(reserved?.has(n) ?? false) || this.files.some((f) => f.name === n && !f.isFolderRoot)
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
