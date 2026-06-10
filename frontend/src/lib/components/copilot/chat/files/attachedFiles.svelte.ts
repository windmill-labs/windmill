/**
 * Conversation-scoped store of files the user has attached to the GLOBAL AI chat.
 *
 * Files are held as `File` handles (lazy disk references) plus a line-offset index
 * built lazily on attach. Content is never decoded wholesale — the read/search tools
 * stream slices on demand (see ./fileEngine). The store lives on AIChatManager and is
 * cleared when a new conversation starts; a page reload wipes it (handles die).
 */
import { buildLineIndex, isTextFile, type FileEntry } from './fileEngine'

export interface AttachedFile extends FileEntry {
	size: number
	status: 'indexing' | 'ready' | 'error'
	error?: string
	/** Top-level folder this file came from (first path segment), if added as part of a folder. */
	folder?: string
}

export interface AddFilesResult {
	added: string[]
	rejected: { name: string; reason: string }[]
}

/** A file to attach, optionally with a relative path to use as its display name (folder adds). */
export type FileToAttach = File | { file: File; path?: string }

/** Cap on the number of attached files, to keep the roster (and RAM) sane. */
export const MAX_ATTACHED_FILES = 100

export class AttachedFilesStore {
	files = $state<AttachedFile[]>([])

	list(): AttachedFile[] {
		return this.files
	}

	get(name: string): AttachedFile | undefined {
		return this.files.find((f) => f.name === name)
	}

	/** Files that have finished indexing and are usable by the tools. */
	readyFiles(): AttachedFile[] {
		return this.files.filter((f) => f.status === 'ready')
	}

	get count(): number {
		return this.files.length
	}

	clear(): void {
		this.files = []
	}

	removeFile(name: string): void {
		this.files = this.files.filter((f) => f.name !== name)
	}

	/** Remove every file that was attached as part of the given folder. */
	removeFolder(folder: string): void {
		this.files = this.files.filter((f) => f.folder !== folder)
	}

	/**
	 * Attach files. Each item is either a raw File (named by its filename, or by
	 * `webkitRelativePath` when it came from a folder picker) or `{ file, path }`
	 * where `path` is the relative path to use as the display name (folder drops).
	 * Rejects binaries and over-cap files (reported in the result so the caller can
	 * toast). Identical re-attachments are no-ops. Names are made unique. Indexing
	 * runs async.
	 */
	async addFiles(input: FileList | FileToAttach[]): Promise<AddFilesResult> {
		const result: AddFilesResult = { added: [], rejected: [] }

		for (const item of Array.from(input as ArrayLike<FileToAttach>)) {
			const file = item instanceof File ? item : item.file
			const desired =
				(item instanceof File ? '' : (item.path ?? '')) ||
				(file as File & { webkitRelativePath?: string }).webkitRelativePath ||
				file.name ||
				'file'

			if (this.files.length >= MAX_ATTACHED_FILES) {
				result.rejected.push({
					name: desired,
					reason: `Attached-file limit (${MAX_ATTACHED_FILES}) reached`
				})
				continue
			}

			// Re-attachment of an already-present path or identical file → no-op.
			if (
				this.files.some(
					(f) =>
						f.name === desired ||
						(f.file.name === file.name &&
							f.size === file.size &&
							f.file.lastModified === file.lastModified)
				)
			) {
				continue
			}

			let textual: boolean
			try {
				textual = await isTextFile(file)
			} catch {
				textual = false
			}
			if (!textual) {
				result.rejected.push({ name: desired, reason: 'Not a text file' })
				continue
			}

			const name = this.#uniqueName(desired)
			const folder = desired.includes('/') ? desired.split('/')[0] : undefined
			this.files = [
				...this.files,
				{ name, file, size: file.size, lineIndex: [], lineCount: 0, status: 'indexing', folder }
			]
			result.added.push(name)
			void this.#indexFile(name, file)
		}

		return result
	}

	async #indexFile(name: string, file: File): Promise<void> {
		try {
			const { lineIndex, lineCount } = await buildLineIndex(file)
			this.#patch(name, { lineIndex, lineCount, status: 'ready' })
		} catch (e) {
			this.#patch(name, {
				status: 'error',
				error: e instanceof Error ? e.message : String(e)
			})
		}
	}

	/** Reassign the array so reactivity fires regardless of proxy identity. */
	#patch(name: string, changes: Partial<AttachedFile>): void {
		this.files = this.files.map((f) => (f.name === name ? { ...f, ...changes } : f))
	}

	#uniqueName(original: string): string {
		if (!this.files.some((f) => f.name === original)) return original
		const dot = original.lastIndexOf('.')
		const base = dot > 0 ? original.slice(0, dot) : original
		const ext = dot > 0 ? original.slice(dot) : ''
		let n = 2
		let candidate = `${base} (${n})${ext}`
		while (this.files.some((f) => f.name === candidate)) {
			n++
			candidate = `${base} (${n})${ext}`
		}
		return candidate
	}
}
