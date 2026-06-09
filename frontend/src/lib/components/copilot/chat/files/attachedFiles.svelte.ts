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
}

export interface AddFilesResult {
	added: string[]
	rejected: { name: string; reason: string }[]
}

/** Cap on the number of attached files, to keep the roster (and RAM) sane. */
export const MAX_ATTACHED_FILES = 25

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

	/**
	 * Attach files. Rejects binaries and over-cap files (reported in the result so
	 * the caller can toast). Identical re-drops are no-ops. Names are made unique
	 * (auto-suffixed) since the tools address files by name. Indexing runs async.
	 */
	async addFiles(input: FileList | File[]): Promise<AddFilesResult> {
		const result: AddFilesResult = { added: [], rejected: [] }

		for (const file of Array.from(input)) {
			if (this.files.length >= MAX_ATTACHED_FILES) {
				result.rejected.push({
					name: file.name,
					reason: `Limit of ${MAX_ATTACHED_FILES} attached files reached`
				})
				continue
			}

			// Identical re-drop of an already-attached file → no-op.
			if (
				this.files.some(
					(f) =>
						f.file.name === file.name &&
						f.size === file.size &&
						f.file.lastModified === file.lastModified
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
				result.rejected.push({ name: file.name, reason: 'Not a text file' })
				continue
			}

			const name = this.#uniqueName(file.name || 'file')
			this.files = [
				...this.files,
				{ name, file, size: file.size, lineIndex: [], lineCount: 0, status: 'indexing' }
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
