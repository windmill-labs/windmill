// The pipelines last left in the assets-only view, as `<workspace>/<folder>`, so reopening one
// shows it the same way. Browser-local: a missing or unreadable list reads as
// "none", and storage that throws (private mode, blocked site data) is ignored.
const KEY = 'pipeline-assets-only-folders'

function read(): string[] {
	try {
		const parsed = JSON.parse(localStorage.getItem(KEY) ?? '[]')
		return Array.isArray(parsed) ? parsed.filter((f) => typeof f === 'string') : []
	} catch {
		return []
	}
}

export function isAssetsOnlyFolder(folder: string): boolean {
	return read().includes(folder)
}

export function setAssetsOnlyFolder(folder: string, on: boolean): void {
	const rest = read().filter((f) => f !== folder)
	try {
		localStorage.setItem(KEY, JSON.stringify(on ? [...rest, folder] : rest))
	} catch {}
}
