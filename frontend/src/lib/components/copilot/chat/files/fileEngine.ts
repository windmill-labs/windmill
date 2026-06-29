/**
 * Storage-agnostic streaming engine for reading and searching attached files.
 *
 * Files are kept as `File` handles (lazy references to bytes on disk). Nothing is
 * decoded into the JS heap wholesale: we stream in chunks, so a large file never
 * freezes the tab or blows up memory. The only per-file state held in RAM is a
 * line-offset index (a flat array of byte offsets, ~8 bytes per line).
 *
 * Line semantics match `String.split('\n')` except a single trailing newline does
 * NOT add an empty final line (so "a\nb\n" is 2 lines, like `wc -l`). Lines are
 * 1-based in the public read/search API.
 */

/** Minimal shape the engine needs. The attached-files store extends this with reactive status. */
export interface FileEntry {
	name: string
	/** A File (live link) or a Blob (restored snapshot) — both stream/slice identically. */
	file: File | Blob
	lineIndex: number[]
	lineCount: number
}

const CHUNK_NEWLINE = 0x0a // '\n' — in UTF-8 this byte never appears inside a multibyte sequence

export const DEFAULT_READ_MAX_LINES = 200
export const DEFAULT_READ_MAX_CHARS = 8000
export const DEFAULT_SEARCH_MAX_HITS = 50
/**
 * Per-line cap on how much of a degenerate long line the regex is tested against.
 * This bounds work for linear-time patterns; it does NOT prevent catastrophic
 * backtracking — a nested-quantifier pattern can still go exponential within the
 * capped prefix (search runs on the main thread, so that is a self-DoS of the tab).
 */
export const DEFAULT_SEARCH_LINE_SCAN_CAP = 100_000
/** How much of a matching line we echo back, to keep search results bounded. */
export const DEFAULT_SEARCH_LINE_ECHO_CAP = 500

/**
 * Stream the file once and record the byte offset at which each line starts.
 * Scans raw bytes for '\n' (no decode needed — 0x0A is unambiguous in UTF-8).
 */
export async function buildLineIndex(
	file: Blob
): Promise<{ lineIndex: number[]; lineCount: number }> {
	const fileSize = file.size
	if (fileSize === 0) {
		return { lineIndex: [], lineCount: 0 }
	}

	const lineIndex: number[] = [0]
	let offset = 0
	const reader = file.stream().getReader()
	try {
		while (true) {
			const { done, value } = await reader.read()
			if (done) break
			const chunk = value as Uint8Array
			for (let i = 0; i < chunk.length; i++) {
				if (chunk[i] === CHUNK_NEWLINE) {
					lineIndex.push(offset + i + 1)
				}
			}
			offset += chunk.length
		}
	} finally {
		reader.releaseLock()
	}

	// A trailing newline points one past the end (a phantom empty line) — drop it.
	if (lineIndex.length > 1 && lineIndex[lineIndex.length - 1] === fileSize) {
		lineIndex.pop()
	}

	return { lineIndex, lineCount: lineIndex.length }
}

export interface ReadResult {
	text: string
	startLine: number
	endLine: number
	totalLines: number
	truncated: boolean
	note: string
}

/**
 * Read a bounded window of lines, reading only the relevant byte range from disk.
 * Clamps the window to `maxLines` and the returned text to `maxChars` (protects
 * against degenerate single-line files). Returns a self-describing pagination note.
 */
export async function readFile(
	entry: FileEntry,
	opts: {
		startLine?: number
		endLine?: number
		maxLines?: number
		maxChars?: number
	} = {}
): Promise<ReadResult> {
	const totalLines = entry.lineCount
	const maxLines = opts.maxLines ?? DEFAULT_READ_MAX_LINES
	const maxChars = opts.maxChars ?? DEFAULT_READ_MAX_CHARS

	if (totalLines === 0) {
		return {
			text: '',
			startLine: 0,
			endLine: 0,
			totalLines: 0,
			truncated: false,
			note: 'File is empty.'
		}
	}

	let start = opts.startLine ?? 1
	if (start < 1) start = 1
	if (start > totalLines) start = totalLines

	const requestedEnd = opts.endLine ?? start + maxLines - 1
	let end = requestedEnd
	if (end < start) end = start
	const cappedByLines = end - start + 1 > maxLines
	if (cappedByLines) end = start + maxLines - 1
	if (end > totalLines) end = totalLines

	const byteStart = entry.lineIndex[start - 1]
	const byteEnd = end < totalLines ? entry.lineIndex[end] : entry.file.size
	// Bound the decode for newline-sparse files (minified JS, single-line JSONL): the
	// window can span the whole file, but we only ever return maxChars characters, and
	// a UTF-8 character is at most 4 bytes — so never materialize more than that.
	const byteCap = byteStart + maxChars * 4
	const byteCapped = byteCap < byteEnd

	let text: string
	try {
		text = await entry.file.slice(byteStart, byteCapped ? byteCap : byteEnd).text()
	} catch (e) {
		throw new FileReadError(entry.name, e instanceof Error ? e.message : String(e))
	}

	let cappedByChars = byteCapped
	if (text.length > maxChars) {
		text = text.slice(0, maxChars)
		cappedByChars = true
	}

	// When the char cap truncates the window short of `end`, the text holds fewer lines
	// than requested — so the note must report the last line actually returned and resume
	// at the next unread one (otherwise it claims lines it didn't return and skips them).
	let lastLine = end
	let resumeAt: number | undefined = end < totalLines ? end + 1 : undefined
	if (cappedByChars) {
		const completeLines = (text.match(/\n/g) || []).length
		if (completeLines >= 1) {
			// lines start..start+completeLines-1 are whole; the next line was cut mid-content.
			// Trim that partial line off the returned text so the body matches the note (and
			// the model doesn't see a line the note says it'll get on the next read).
			lastLine = start + completeLines - 1
			resumeAt = start + completeLines
			text = text.slice(0, text.lastIndexOf('\n') + 1)
		} else {
			// the cap fell inside line `start` itself — it can't be returned in full, so
			// advance past it rather than re-truncating the same line forever.
			lastLine = start
			resumeAt = start + 1
		}
		if (resumeAt > totalLines) resumeAt = undefined
	}

	const truncated = cappedByChars || resumeAt !== undefined

	let note = `Showing lines ${start}-${lastLine} of ${totalLines}.`
	if (cappedByChars) {
		note += ` Output truncated to ${maxChars} characters (line(s) very long).`
	}
	if (resumeAt !== undefined) {
		note += ` Call read_file again with start_line=${resumeAt} for more.`
	}

	return { text, startLine: start, endLine: lastLine, totalLines, truncated, note }
}

/**
 * Prefix each line of a read window with its absolute 1-based number (`<n>→<content>`),
 * so the model can quote/reference exact lines. `startLine` is the window's first line.
 */
export function numberLines(text: string, startLine: number): string {
	const lines = text.split('\n')
	// readFile's window ends with the trailing newline of its last line when more lines
	// follow, so split yields a phantom empty element — drop it before numbering.
	if (lines.length > 1 && lines[lines.length - 1] === '') lines.pop()
	const width = String(startLine + lines.length - 1).length
	return lines.map((l, i) => `${String(startLine + i).padStart(width)}→${l}`).join('\n')
}

export interface SearchHit {
	file: string
	line: number
	text: string
}

export interface SearchResult {
	hits: SearchHit[]
	truncated: boolean
	error?: string
}

/**
 * Run a regex across one or more files, streaming each (no full-file load), and
 * return matching lines with 1-based line numbers. Stops at `maxHits`.
 */
export async function searchFiles(
	entries: FileEntry[],
	pattern: string,
	opts: {
		flags?: string
		pathFilter?: string
		maxHits?: number
		lineScanCap?: number
		lineEchoCap?: number
	} = {}
): Promise<SearchResult> {
	const maxHits = opts.maxHits ?? DEFAULT_SEARCH_MAX_HITS
	const lineScanCap = opts.lineScanCap ?? DEFAULT_SEARCH_LINE_SCAN_CAP
	const lineEchoCap = opts.lineEchoCap ?? DEFAULT_SEARCH_LINE_ECHO_CAP

	let regex: RegExp
	try {
		regex = new RegExp(pattern, opts.flags ?? '')
	} catch (e) {
		return {
			hits: [],
			truncated: false,
			error: `Invalid regex: ${e instanceof Error ? e.message : String(e)}`
		}
	}

	const targets = opts.pathFilter ? entries.filter((e) => e.name === opts.pathFilter) : entries
	if (opts.pathFilter && targets.length === 0) {
		return { hits: [], truncated: false, error: `No attached file named "${opts.pathFilter}".` }
	}

	const hits: SearchHit[] = []
	let truncated = false

	for (const entry of targets) {
		if (hits.length >= maxHits) {
			truncated = true
			break
		}
		try {
			await streamLines(entry.file, (line, lineNo) => {
				// Bound backtracking on pathological long lines by only testing a prefix.
				const scanned = line.length > lineScanCap ? line.slice(0, lineScanCap) : line
				// `regex` may carry a caller-supplied `g`/`y` flag, which makes `.test()`
				// stateful (it advances `lastIndex`) — reset so each line matches from 0.
				regex.lastIndex = 0
				if (regex.test(scanned)) {
					hits.push({
						file: entry.name,
						line: lineNo,
						text: line.length > lineEchoCap ? line.slice(0, lineEchoCap) + '…' : line
					})
				}
				return hits.length < maxHits // continue?
			})
		} catch (e) {
			return {
				hits,
				truncated,
				error: `Error reading "${entry.name}": ${e instanceof Error ? e.message : String(e)}`
			}
		}
		if (hits.length >= maxHits) {
			truncated = true
			break
		}
	}

	return { hits, truncated }
}

/**
 * Run `searchFiles` off the main thread. A model-supplied regex can backtrack
 * catastrophically (e.g. /^(a+)+$/) and `regex.test()` can't be interrupted — so we run
 * it in a Worker and `terminate()` it on a timeout, keeping the tab responsive instead of
 * frozen. Falls back to a main-thread search where Workers aren't available (best effort).
 */
export function searchFilesInWorker(
	entries: FileEntry[],
	pattern: string,
	opts: { flags?: string; pathFilter?: string; maxHits?: number } = {},
	timeoutMs = 3000
): Promise<SearchResult> {
	let worker: Worker
	try {
		worker = new Worker(new URL('./searchWorker.ts', import.meta.url), { type: 'module' })
	} catch {
		return searchFiles(entries, pattern, opts)
	}
	return new Promise<SearchResult>((resolve) => {
		const finish = (r: SearchResult) => {
			clearTimeout(timer)
			worker.terminate()
			resolve(r)
		}
		const timer = setTimeout(
			() =>
				finish({
					hits: [],
					truncated: false,
					error: 'Search timed out — the pattern is too expensive. Try a simpler regex.'
				}),
			timeoutMs
		)
		worker.onmessage = (e: MessageEvent<SearchResult>) => finish(e.data)
		worker.onerror = () => {
			// Worker script failed to load/run — fall back to a main-thread search.
			clearTimeout(timer)
			worker.terminate()
			searchFiles(entries, pattern, opts).then(resolve)
		}
		worker.postMessage({
			files: entries.map((e) => ({ name: e.name, file: e.file })),
			pattern,
			flags: opts.flags,
			pathFilter: opts.pathFilter,
			maxHits: opts.maxHits
		})
	})
}

export class FileReadError extends Error {
	constructor(
		public fileName: string,
		message: string
	) {
		super(message)
		this.name = 'FileReadError'
	}
}

/**
 * Max characters buffered for a single line while streaming. A newline-less file
 * (e.g. minified JS) would otherwise accumulate wholesale in `buffer`; past this
 * cap excess characters are dropped (the line's intact prefix is preserved) —
 * harmless for search, which only tests/echoes a prefix far smaller than this.
 */
const MAX_LINE_BUFFER_CHARS = 1_000_000

/**
 * Stream a file and invoke `onLine` for each line (1-based). A trailing newline
 * does not produce an empty final line. `onLine` returns false to stop early.
 * A trailing '\r' (CRLF) is stripped before the callback. Overlong lines are
 * passed with at least their first MAX_LINE_BUFFER_CHARS characters intact;
 * content beyond the cap may be dropped.
 */
async function streamLines(
	file: Blob,
	onLine: (line: string, lineNo: number) => boolean
): Promise<void> {
	const reader = file.stream().getReader()
	const decoder = new TextDecoder('utf-8')
	let buffer = ''
	let lineNo = 0
	try {
		while (true) {
			const { done, value } = await reader.read()
			if (done) {
				buffer += decoder.decode()
				break
			}
			buffer += decoder.decode(value as Uint8Array, { stream: true })
			let start = 0
			let nlIdx: number
			while ((nlIdx = buffer.indexOf('\n', start)) !== -1) {
				let line = buffer.slice(start, nlIdx)
				if (line.endsWith('\r')) line = line.slice(0, -1)
				start = nlIdx + 1
				lineNo++
				if (!onLine(line, lineNo)) return
			}
			buffer = buffer.slice(start)
			// The remainder holds no newline — cap how much of an overlong line we keep.
			// Dropped characters are line content only, so newline detection and line
			// numbering in later chunks are unaffected.
			if (buffer.length > MAX_LINE_BUFFER_CHARS) {
				buffer = buffer.slice(0, MAX_LINE_BUFFER_CHARS)
			}
		}
		if (buffer.length > 0) {
			let line = buffer
			if (line.endsWith('\r')) line = line.slice(0, -1)
			lineNo++
			onLine(line, lineNo)
		}
	} finally {
		reader.releaseLock()
	}
}

/**
 * Sniff the first bytes of a file to decide whether it is text (UTF-8 decodable,
 * no NUL bytes). Used to reject binary files at attach time.
 */
export async function isTextFile(file: Blob, sampleBytes = 8192): Promise<boolean> {
	if (file.size === 0) return true
	const slice = file.slice(0, Math.min(sampleBytes, file.size))
	const buf = new Uint8Array(await slice.arrayBuffer())
	for (let i = 0; i < buf.length; i++) {
		if (buf[i] === 0) return false // NUL byte → binary
	}
	try {
		// `fatal` throws on invalid UTF-8. We may cut a multibyte char at the sample
		// boundary, so only treat it as binary if the error is not at the very end.
		new TextDecoder('utf-8', { fatal: true }).decode(buf)
		return true
	} catch {
		// Could be a truncated trailing multibyte sequence — retry on a trimmed buffer.
		if (buf.length >= 4) {
			try {
				new TextDecoder('utf-8', { fatal: true }).decode(buf.slice(0, buf.length - 3))
				return true
			} catch {
				return false
			}
		}
		return false
	}
}
