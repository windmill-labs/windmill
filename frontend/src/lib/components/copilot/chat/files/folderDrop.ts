/**
 * Folder support for attached files: recurse dropped directory entries and filter
 * a folder-picker FileList into a flat list of `{ file, path }`, skipping junk
 * (dotfiles/dotdirs, node_modules, build output, …) and binaries (the latter via
 * the content sniff in the store). Files keep their relative path as a display name.
 */
import type { FileToAttach } from './attachedFiles.svelte'

const IGNORED_DIRS = new Set([
	'node_modules',
	'dist',
	'build',
	'out',
	'target',
	'vendor',
	'coverage',
	'__pycache__',
	'.git',
	'.svelte-kit',
	'.next',
	'.nuxt',
	'.venv',
	'venv',
	'.idea',
	'.vscode',
	'.turbo',
	'.cache'
])

/** Max files collected from a single folder add, to avoid pathological directories. */
export const MAX_FOLDER_FILES = 500

function isIgnoredSegment(name: string): boolean {
	return name.startsWith('.') || IGNORED_DIRS.has(name)
}

/** True if any segment of a relative path is a dotfile/dotdir or an ignored directory. */
export function isIgnoredPath(path: string): boolean {
	return path.split('/').some(isIgnoredSegment)
}

function readEntriesBatch(reader: FileSystemDirectoryReader): Promise<FileSystemEntry[]> {
	return new Promise((resolve, reject) => reader.readEntries(resolve, reject))
}

function entryFile(entry: FileSystemFileEntry): Promise<File> {
	return new Promise((resolve, reject) => entry.file(resolve, reject))
}

/**
 * Recurse pre-captured drop entries into a flat list of files with relative paths.
 * Top-level dropped files are kept as-is (no junk filtering, for parity with the
 * file picker); files found by recursing into a folder are junk-filtered.
 *
 * Entries MUST be captured synchronously in the drop handler (via
 * `webkitGetAsEntry()`) before awaiting this function.
 */
export async function collectDroppedEntries(entries: FileSystemEntry[]): Promise<FileToAttach[]> {
	const out: FileToAttach[] = []

	async function walkDir(dir: FileSystemDirectoryEntry, base: string): Promise<void> {
		const reader = dir.createReader()
		// readEntries returns in batches (~100); loop until it returns nothing.
		while (out.length < MAX_FOLDER_FILES) {
			const batch = await readEntriesBatch(reader)
			if (batch.length === 0) break
			for (const child of batch) {
				if (out.length >= MAX_FOLDER_FILES) break
				const path = `${base}/${child.name}`
				if (child.isFile) {
					if (isIgnoredPath(path)) continue
					out.push({ file: await entryFile(child as FileSystemFileEntry), path })
				} else if (child.isDirectory) {
					if (isIgnoredSegment(child.name)) continue
					await walkDir(child as FileSystemDirectoryEntry, path)
				}
			}
		}
	}

	for (const entry of entries) {
		if (out.length >= MAX_FOLDER_FILES) break
		if (entry.isFile) {
			out.push({ file: await entryFile(entry as FileSystemFileEntry), path: entry.name })
		} else if (entry.isDirectory) {
			if (isIgnoredSegment(entry.name)) continue
			await walkDir(entry as FileSystemDirectoryEntry, entry.name)
		}
	}
	return out
}

/** Filter a folder-picker (`webkitdirectory`) FileList, junk-filtering by relative path. */
export function filterFolderPickerFiles(files: FileList): FileToAttach[] {
	const out: FileToAttach[] = []
	for (const file of Array.from(files)) {
		const path = (file as File & { webkitRelativePath?: string }).webkitRelativePath || file.name
		if (isIgnoredPath(path)) continue
		out.push({ file, path })
		if (out.length >= MAX_FOLDER_FILES) break
	}
	return out
}
