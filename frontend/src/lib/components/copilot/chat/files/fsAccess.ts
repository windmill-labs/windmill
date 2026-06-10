/**
 * Thin wrappers over the File System Access API, used when available so linked
 * files/folders can be re-read live after a reload (re-grantable handles).
 * Capability is feature-detected (never browser-sniffed): the day Firefox/Safari
 * ship the API, the handle path lights up automatically.
 */
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

/** Max files collected from a single folder, to avoid pathological directories. */
export const MAX_FOLDER_FILES = 500

function isIgnoredSegment(name: string): boolean {
	return name.startsWith('.') || IGNORED_DIRS.has(name)
}

/** True if any segment of a relative path is a dotfile/dotdir or an ignored directory. */
export function isIgnoredPath(path: string): boolean {
	return path.split('/').some(isIgnoredSegment)
}

type FSWindow = Window & {
	showDirectoryPicker?: (opts?: {
		mode?: 'read' | 'readwrite'
	}) => Promise<FileSystemDirectoryHandle>
}

type FSDataTransferItem = DataTransferItem & {
	getAsFileSystemHandle?: () => Promise<FileSystemHandle | null>
}

/**
 * True when the File System Access API needed for FOLDER linking is usable:
 * the directory picker plus drag-drop handles. (Files never use the API — they're
 * always snapshotted — so showOpenFilePicker is intentionally not required.)
 */
export function hasFileSystemAccess(): boolean {
	return (
		typeof window !== 'undefined' &&
		'showDirectoryPicker' in window &&
		typeof DataTransferItem !== 'undefined' &&
		'getAsFileSystemHandle' in DataTransferItem.prototype
	)
}

/** Open the directory picker. Returns undefined if the user cancels. */
export async function pickDirectory(): Promise<FileSystemDirectoryHandle | undefined> {
	const w = window as FSWindow
	if (!w.showDirectoryPicker) return undefined
	try {
		return await w.showDirectoryPicker({ mode: 'read' })
	} catch {
		return undefined
	}
}

/**
 * Resolve File System Access handles from a drop's items. The `getAsFileSystemHandle`
 * calls are kicked off synchronously (items are only valid during the drop event);
 * the returned promise resolves the handles.
 */
export function handlesFromDataTransfer(dt: DataTransfer): Promise<FileSystemHandle[]> {
	const pending = Array.from(dt.items)
		.filter((it) => it.kind === 'file')
		.map((it) => (it as FSDataTransferItem).getAsFileSystemHandle?.() ?? Promise.resolve(null))
	return Promise.all(pending).then((handles) => handles.filter((h): h is FileSystemHandle => !!h))
}

export function isFileHandle(h: FileSystemHandle): h is FileSystemFileHandle {
	return h.kind === 'file'
}
export function isDirectoryHandle(h: FileSystemHandle): h is FileSystemDirectoryHandle {
	return h.kind === 'directory'
}

/**
 * Recursively read a directory handle into a flat list of files with relative paths,
 * skipping junk (dotfiles/dotdirs, node_modules, …) and capping the count. Used both
 * on link and on live re-enumeration after a reload.
 */
export async function enumerateDir(
	dir: FileSystemDirectoryHandle
): Promise<{ file: File; path: string }[]> {
	const out: { file: File; path: string }[] = []

	async function walk(handle: FileSystemDirectoryHandle, prefix: string): Promise<void> {
		// @ts-ignore - values() is an async iterator in the File System Access API
		for await (const entry of handle.values() as AsyncIterable<FileSystemHandle>) {
			if (out.length >= MAX_FOLDER_FILES) return
			const path = `${prefix}/${entry.name}`
			if (isIgnoredPath(path)) continue
			if (isFileHandle(entry)) {
				out.push({ file: await entry.getFile(), path })
			} else if (isDirectoryHandle(entry)) {
				await walk(entry, path)
			}
		}
	}

	await walk(dir, dir.name)
	return out
}

/** queryPermission without a user gesture; 'granted' | 'prompt' | 'denied'. */
export async function queryReadPermission(handle: FileSystemHandle): Promise<PermissionState> {
	// @ts-ignore - queryPermission is part of the File System Access API
	return (await handle.queryPermission?.({ mode: 'read' })) ?? 'prompt'
}

/** requestPermission — MUST be called within a user gesture. */
export async function requestReadPermission(handle: FileSystemHandle): Promise<PermissionState> {
	// @ts-ignore - requestPermission is part of the File System Access API
	return (await handle.requestPermission?.({ mode: 'read' })) ?? 'denied'
}
