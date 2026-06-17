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

/**
 * Open the directory picker. Returns undefined if the user dismisses it.
 * Any other failure (a policy that blocks the File System Access API, a lost
 * user-activation, etc.) is rethrown — swallowing it makes the picker silently
 * never open, which is indistinguishable from a no-op and impossible to debug.
 */
export async function pickDirectory(): Promise<FileSystemDirectoryHandle | undefined> {
	const w = window as FSWindow
	if (!w.showDirectoryPicker) return undefined
	try {
		return await w.showDirectoryPicker({ mode: 'read' })
	} catch (e) {
		// AbortError means the user dismissed the dialog (and, under browser automation,
		// that CDP intercepted the chooser) — a no-op, not a failure.
		if (e instanceof DOMException && e.name === 'AbortError') return undefined
		throw e
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
 * skipping junk (dotfiles/dotdirs, node_modules, …). No file-count cap — the browser's
 * memory/quota are the only limit. Used on link and on live re-enumeration after a reload.
 */
export async function enumerateDir(
	dir: FileSystemDirectoryHandle
): Promise<{ file: File; path: string }[]> {
	const out: { file: File; path: string }[] = []

	async function walk(handle: FileSystemDirectoryHandle, prefix: string): Promise<void> {
		// @ts-ignore - values() is an async iterator in the File System Access API
		for await (const entry of handle.values() as AsyncIterable<FileSystemHandle>) {
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

/**
 * Recursively read dropped files AND folders via the legacy `webkitGetAsEntry` API —
 * the fallback for browsers without the File System Access API (Firefox/Safari). Folder
 * contents are snapshotted into the browser (no live handle). Each result `path` is
 * folder-relative (`folder/sub/file` for a dropped folder, bare name for a loose file),
 * junk paths skipped, no file-count cap.
 *
 * `webkitGetAsEntry()` is only valid synchronously during the drop event, so this MUST be
 * called from the drop handler — its `.map(...)` runs before the first `await`, capturing
 * the entries while the items are still live.
 */
export async function readDroppedEntries(
	items: DataTransferItem[]
): Promise<{ file: File; path: string }[]> {
	const roots = items
		.map((it) => it.webkitGetAsEntry?.() ?? null)
		.filter((e): e is FileSystemEntry => !!e)
	const out: { file: File; path: string }[] = []
	for (const root of roots) await walkDropEntry(root, out)
	return out
}

async function walkDropEntry(
	entry: FileSystemEntry,
	out: { file: File; path: string }[]
): Promise<void> {
	const path = entry.fullPath.replace(/^\//, '')
	if (isIgnoredPath(path)) return
	if (entry.isFile) {
		const fileEntry = entry as FileSystemFileEntry
		const file = await new Promise<File>((res, rej) => fileEntry.file(res, rej))
		out.push({ file, path })
	} else if (entry.isDirectory) {
		const reader = (entry as FileSystemDirectoryEntry).createReader()
		// readEntries yields in batches and returns [] once exhausted — loop until empty.
		while (true) {
			const batch = await new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej))
			if (batch.length === 0) break
			for (const child of batch) await walkDropEntry(child, out)
		}
	}
}

/** queryPermission without a user gesture; 'granted' | 'prompt' | 'denied'. Never rejects. */
export async function queryReadPermission(handle: FileSystemHandle): Promise<PermissionState> {
	try {
		// @ts-ignore - queryPermission is part of the File System Access API
		return (await handle.queryPermission?.({ mode: 'read' })) ?? 'prompt'
	} catch {
		return 'prompt'
	}
}

/**
 * requestPermission — MUST be called within a user gesture. Never rejects: the spec
 * rejects with SecurityError when user activation is missing (e.g. a second prompt
 * after the first consumed the gesture) — that maps to 'denied' here so callers can
 * treat it as "still locked" instead of blowing up the send path.
 */
export async function requestReadPermission(handle: FileSystemHandle): Promise<PermissionState> {
	try {
		// @ts-ignore - requestPermission is part of the File System Access API
		return (await handle.requestPermission?.({ mode: 'read' })) ?? 'denied'
	} catch {
		return 'denied'
	}
}
