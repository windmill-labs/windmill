/**
 * Web Worker that runs `search_files` off the main thread.
 *
 * The regex is model-supplied and `RegExp.prototype.test()` can't be interrupted, so a
 * catastrophic-backtracking pattern (e.g. /^(a+)+$/) would otherwise hang the whole tab.
 * Running it here lets the caller `terminate()` this worker on a timeout instead. The
 * matching itself reuses `searchFiles` from the engine (single source of truth).
 */
import { searchFiles, type FileEntry } from './fileEngine'

interface SearchRequest {
	files: { name: string; file: Blob }[]
	pattern: string
	flags?: string
	pathFilter?: string
	maxHits?: number
}

self.onmessage = async (e: MessageEvent<SearchRequest>) => {
	const { files, pattern, flags, pathFilter, maxHits } = e.data
	// searchFiles only reads `name` + `file` (it streams); the index fields are unused here.
	const entries: FileEntry[] = files.map((f) => ({
		name: f.name,
		file: f.file,
		lineIndex: [],
		lineCount: 0
	}))
	try {
		const result = await searchFiles(entries, pattern, { flags, pathFilter, maxHits })
		;(self as unknown as Worker).postMessage(result)
	} catch (err) {
		;(self as unknown as Worker).postMessage({
			hits: [],
			truncated: false,
			error: err instanceof Error ? err.message : String(err)
		})
	}
}
