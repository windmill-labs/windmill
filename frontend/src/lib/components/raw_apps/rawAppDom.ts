/**
 * Live DOM inspection for a raw-app preview, exposed to the AI session chat.
 *
 * The rendered app is NEVER inlined into the model's context. Instead a selected
 * element is referenced by its CSS selector, and the model pulls only the slices it
 * needs through the `search_dom` / `read_dom` tools — mirroring the attached-files
 * design (see ../copilot/chat/files/fileTools.ts). The element's current `outerHTML`
 * is re-read live on every call (host-side, same-origin), pretty-printed to lines,
 * and run through the same bounded, worker-guarded engine the file tools use.
 */
import {
	buildLineIndex,
	numberLines,
	readFile,
	searchFilesInWorker,
	type FileEntry,
	type SearchHit
} from '../copilot/chat/files/fileEngine'

/** Regex search over a selected element's rendered HTML. */
export type RawAppDomSearchQuery = {
	mode: 'search'
	/** Raw-app path to read from. Routes the query to that app's (still-mounted)
	 * preview even when another tab is visible; omit to use the active preview. */
	appPath?: string
	/** CSS selector; omit for the whole `document.body`. */
	selector?: string
	pattern: string
	ignoreCase?: boolean
}

/** Bounded line-window read of a selected element's rendered HTML. */
export type RawAppDomReadQuery = {
	mode: 'read'
	appPath?: string
	selector?: string
	startLine?: number
	endLine?: number
}

export type RawAppDomQuery = RawAppDomSearchQuery | RawAppDomReadQuery

/** Formatted, model-ready text for a single DOM query. */
export type RawAppDomResult = { text: string }

/**
 * Resolves a DOM query against the live preview. Returns `undefined` only when no
 * preview is loaded (so the caller can distinguish "no running app" from "selector
 * matched nothing", which is itself a useful, returned result).
 */
export type RawAppDomRequester = (query: RawAppDomQuery) => Promise<RawAppDomResult | undefined>

/**
 * Break a (typically single-line) `outerHTML` string into indented lines so the
 * line-oriented file engine can search/read it meaningfully. Deliberately simple:
 * one line per tag boundary, indented by nesting depth. It is not a faithful
 * re-serializer (whitespace-sensitive nodes like <pre>/<textarea> are reflowed) —
 * it exists only to give the model greppable, line-numbered structure.
 */
export function prettyPrintHtml(html: string): string {
	// Split so every tag starts a new line; text between tags stays on its own line.
	const withBreaks = html.replace(/>\s*</g, '>\n<')
	const rawLines = withBreaks.split('\n')
	const out: string[] = []
	let depth = 0
	for (const raw of rawLines) {
		const line = raw.trim()
		if (line === '') continue
		const isClosing = /^<\//.test(line)
		// A line that opens a tag but doesn't close it on the same line increases depth.
		// Void elements (<img>, <input>, <br>…) have no closing tag, so they must not
		// open a block — otherwise depth drifts and never recovers.
		const opensBlock =
			/^<[a-zA-Z]/.test(line) &&
			!/\/>\s*$/.test(line) &&
			!isClosing &&
			!isVoidElement(line) &&
			!isSelfContainedElement(line)
		if (isClosing) depth = Math.max(0, depth - 1)
		out.push('  '.repeat(depth) + line)
		if (opensBlock) depth += 1
	}
	return out.join('\n')
}

// HTML void elements: serialized by outerHTML without a trailing slash and never
// have a closing tag, so they are leaves for indentation purposes.
const VOID_ELEMENTS = new Set([
	'area',
	'base',
	'br',
	'col',
	'embed',
	'hr',
	'img',
	'input',
	'link',
	'meta',
	'param',
	'source',
	'track',
	'wbr'
])

function isVoidElement(line: string): boolean {
	const open = line.match(/^<([a-zA-Z][\w-]*)/)
	return !!open && VOID_ELEMENTS.has(open[1].toLowerCase())
}

/** A single line that both opens and closes the same element (e.g. `<span>hi</span>`). */
function isSelfContainedElement(line: string): boolean {
	const open = line.match(/^<([a-zA-Z][\w-]*)/)
	if (!open) return false
	return new RegExp(`</${open[1]}\\s*>\\s*$`).test(line)
}

const DOM_ENTRY_NAME = 'dom'

async function toEntry(prettyHtml: string): Promise<FileEntry> {
	const file = new Blob([prettyHtml], { type: 'text/html' })
	const { lineIndex, lineCount } = await buildLineIndex(file)
	return { name: DOM_ENTRY_NAME, file, lineIndex, lineCount }
}

/** Header describing which element the result is for. */
function header(meta: { selector: string; tagName: string; matchCount: number }): string {
	const scope = meta.selector === 'body' ? 'whole page (<body>)' : `selector "${meta.selector}"`
	const multi =
		meta.matchCount > 1
			? ` (${meta.matchCount} elements match; showing the first, <${meta.tagName}>)`
			: ''
	return `Live DOM for ${scope}${multi}:`
}

/**
 * Run a search/read query against an element's live `outerHTML`. Pure over its
 * inputs (the DOM read happens in the caller) so it is unit-testable without a DOM.
 */
export async function runDomQueryOnHtml(
	html: string,
	query: RawAppDomQuery,
	meta: { selector: string; tagName: string; matchCount: number }
): Promise<string> {
	const entry = await toEntry(prettyPrintHtml(html))
	const head = header(meta)

	if (query.mode === 'search') {
		const result = await searchFilesInWorker([entry], query.pattern, {
			flags: query.ignoreCase ? 'i' : ''
		})
		if (result.error) return `${head}\nError: ${result.error}`
		if (result.hits.length === 0) {
			return `${head}\nNo matches for /${query.pattern}/. Read the element with read_dom to see its structure.`
		}
		const body = result.hits.map((h: SearchHit) => `${h.line}: ${h.text}`).join('\n')
		const footer = result.truncated
			? '\n\n(Stopped at the result limit — refine your pattern.)'
			: ''
		return `${head}\nFound ${result.hits.length} matching line(s):\n${body}${footer}`
	}

	const res = await readFile(entry, { startLine: query.startLine, endLine: query.endLine })
	// readFile's pagination note references the attached-file tool; this wrapper is
	// exposed as read_dom, so rename the continuation instruction to match.
	const note = res.note.replace(/\bread_file\b/g, 'read_dom')
	if (!res.text) return `${head}\n${note}`
	return `${head}\n${note}\n\n${numberLines(res.text, res.startLine)}`
}
