/**
 * AI tools and system-prompt roster for files attached to the GLOBAL chat.
 *
 * The model is made aware of attached files via a metadata-only roster appended to
 * the system message (see `appendAttachedFilesRoster`). Their contents are NEVER
 * inlined — the model pulls only the slices it needs through these two read-only
 * tools, which stream from disk via ./fileEngine.
 */
import { z } from 'zod'
import type { ChatCompletionSystemMessageParam } from 'openai/resources/chat/completions.mjs'
import { createToolDef, type Tool } from '../shared'
import {
	readFile,
	searchFilesInWorker,
	numberLines,
	FileReadError,
	type SearchHit
} from './fileEngine'
import type { AttachedFile, AttachedFilesStore } from './attachedFiles.svelte'

/** Slice of the GLOBAL tool helpers that exposes the attached-files store. */
export interface AttachedFilesHelper {
	attachedFiles?: AttachedFilesStore
}

function storeFrom(helpers: unknown): AttachedFilesStore | undefined {
	return (helpers as AttachedFilesHelper | undefined)?.attachedFiles
}

/**
 * For a specifically requested attached file, a message describing why it can't be read /
 * searched yet (still indexing, locked, unavailable, errored) or that it isn't attached —
 * or undefined when it's `ready`. Shared by read_file and search_files so both report the
 * same accurate status instead of search_files claiming a non-ready file isn't attached.
 */
function notReadyMessage(store: AttachedFilesStore, file: string): string | undefined {
	const entry = store.get(file)
	if (entry?.status === 'ready') return undefined
	if (entry?.status === 'indexing')
		return `File "${file}" is still being indexed. Try again shortly.`
	if (entry?.status === 'locked')
		return `File "${file}" is locked after a reload. Ask the user to restore access (send a message, or click "Restore access").`
	if (entry?.status === 'unavailable')
		return `File "${file}" is no longer available (moved, deleted, or its local copy was evicted). Ask the user to re-link it.`
	if (entry?.status === 'error')
		return `File "${file}" failed to load: ${entry.error ?? 'unknown error'}.`
	const names = store
		.list()
		.map((f) => f.name)
		.join(', ')
	return `No attached file named "${file}". Attached files: ${names || '(none)'}.`
}

/**
 * When attachments exist but none expose a readable target (`readyFiles()` is empty),
 * explain the actual reason instead of always claiming files are still indexing. Empty
 * or binary-only linked folders leave only `ready` placeholder rows (filtered out of
 * `readyFiles`), while a locked/unavailable restore surfaces those statuses on the rows.
 */
function noReadyFilesMessage(store: AttachedFilesStore): string {
	const statuses = new Set(store.list().map((f) => f.status))
	if (statuses.has('indexing')) return 'Attached files are still being indexed. Try again shortly.'
	if (statuses.has('locked'))
		return 'The attached files are locked after a reload. Ask the user to restore access (send a message, or click "Restore access").'
	if (statuses.has('unavailable'))
		return 'The attached files are no longer available (moved, deleted, or their local copies were evicted). Ask the user to re-link them.'
	if (statuses.has('error')) return 'The attached files failed to load.'
	return 'No searchable text files are attached (a linked folder may be empty or contain only non-text files).'
}

function humanSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const searchFilesSchema = z.object({
	pattern: z.string().describe('JavaScript regular expression to search for.'),
	file: z
		.string()
		.optional()
		.describe(
			'Optional exact filename (as listed under "Attached files") to restrict the search to. Omit to search across all attached files.'
		),
	ignore_case: z.boolean().optional().describe('Case-insensitive matching. Defaults to false.')
})

const searchFilesToolDef = createToolDef(
	searchFilesSchema,
	'search_files',
	'Search the user-attached files with a regular expression and return matching lines with their line numbers. Use this to locate content before reading a specific window with read_file.'
)

export const searchFilesTool: Tool<{}> = {
	def: searchFilesToolDef,
	readonly: true,
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = searchFilesSchema.parse(args)
		// Validate a specifically requested file against the full store first, so a non-ready
		// target reports its real status (indexing/locked/…) instead of "not attached".
		if (parsed.file) {
			const notReady = notReadyMessage(store, parsed.file)
			if (notReady) return notReady
		}
		const ready = store.readyFiles()
		if (ready.length === 0) {
			return noReadyFilesMessage(store)
		}
		toolCallbacks.setToolStatus(toolId, {
			content: `Searching attached files for /${parsed.pattern}/...`
		})

		// Run in a Worker so a pathological model-supplied regex can't freeze the tab.
		const result = await searchFilesInWorker(ready, parsed.pattern, {
			flags: parsed.ignore_case ? 'i' : '',
			pathFilter: parsed.file
		})
		if (result.error) {
			return `Error: ${result.error}`
		}
		const scope = parsed.file ? `"${parsed.file}"` : `${ready.length} file(s)`
		if (result.hits.length === 0) {
			return `No matches for /${parsed.pattern}/ in ${scope}.`
		}
		const body = result.hits.map((h: SearchHit) => `${h.file}:${h.line}: ${h.text}`).join('\n')
		const header = `Found ${result.hits.length} match(es) in ${scope}:`
		const footer = result.truncated
			? '\n\n(Stopped at the result limit — refine your pattern or pass a `file` to narrow the search.)'
			: ''
		return `${header}\n${body}${footer}`
	}
}

const readFileSchema = z.object({
	file: z.string().describe('Exact filename to read, as listed under "Attached files".'),
	start_line: z.number().int().optional().describe('1-based first line to read. Defaults to 1.'),
	end_line: z
		.number()
		.int()
		.optional()
		.describe('1-based last line to read. The window is capped at 200 lines.')
})

const readFileToolDef = createToolDef(
	readFileSchema,
	'read_file',
	'Read a bounded window of lines from a user-attached file. Returns each line prefixed with its 1-based number (`<n>→<content>`) plus a pagination note. Files are not in context, so use this to inspect their contents.'
)

export const readFileTool: Tool<{}> = {
	def: readFileToolDef,
	readonly: true,
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = readFileSchema.parse(args)
		const notReady = notReadyMessage(store, parsed.file)
		if (notReady) return notReady
		const entry = store.get(parsed.file)!
		toolCallbacks.setToolStatus(toolId, { content: `Reading "${parsed.file}"...` })

		try {
			const res = await readFile(entry, {
				startLine: parsed.start_line,
				endLine: parsed.end_line
			})
			return res.text ? `${res.note}\n\n${numberLines(res.text, res.startLine)}` : res.note
		} catch (e) {
			if (e instanceof FileReadError) {
				return `Could not read "${parsed.file}": ${e.message}. The file may have been moved or deleted since it was attached.`
			}
			return `Error reading "${parsed.file}": ${e instanceof Error ? e.message : String(e)}`
		}
	}
}

export const fileTools: Tool<{}>[] = [searchFilesTool, readFileTool]

function rosterLine(f: AttachedFile): string {
	if (f.status === 'indexing') return `- ${f.name} (indexing…)`
	if (f.status === 'locked') return `- ${f.name} (locked — needs the user to restore access)`
	if (f.status === 'unavailable') return `- ${f.name} (unavailable)`
	if (f.status === 'error') return `- ${f.name} (failed to load)`
	return `- ${f.name} — ${f.lineCount} lines, ${humanSize(f.size)}`
}

/** Build the `## Attached files` system-prompt section (metadata only, never content). */
export function buildAttachedFilesRoster(store: AttachedFilesStore): string {
	const lines: string[] = []
	for (const folder of store.folders) {
		// A locked/unavailable folder has no readable children — one line for the whole folder.
		if (folder.status === 'locked') {
			lines.push(`- ${folder.name} (locked — needs the user to restore access)`)
		} else if (folder.status === 'unavailable') {
			lines.push(`- ${folder.name} (unavailable)`)
		} else {
			lines.push(...folder.files.map(rosterLine))
		}
	}
	lines.push(...store.standalone.map(rosterLine))
	if (lines.length === 0) return ''
	return [
		'## Attached files',
		'The user has attached the following files to this conversation. Their contents are NOT included here.',
		'Use the `search_files` tool to find content with a regex, and `read_file` to read a bounded window of lines.',
		'',
		lines.join('\n')
	].join('\n')
}

/**
 * Return a copy of the system message with the attached-files roster appended.
 * Always derives from the provided base so the roster never accumulates across turns.
 */
export function appendAttachedFilesRoster(
	base: ChatCompletionSystemMessageParam,
	store: AttachedFilesStore
): ChatCompletionSystemMessageParam {
	const roster = buildAttachedFilesRoster(store)
	if (!roster || typeof base.content !== 'string') return base
	return { ...base, content: `${base.content}\n\n${roster}` }
}
