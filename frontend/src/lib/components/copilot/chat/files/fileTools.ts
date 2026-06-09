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
import { readFile, searchFiles, FileReadError, type SearchHit } from './fileEngine'
import type { AttachedFile, AttachedFilesStore } from './attachedFiles.svelte'

/** Slice of the GLOBAL tool helpers that exposes the attached-files store. */
export interface AttachedFilesHelper {
	attachedFiles?: AttachedFilesStore
}

function storeFrom(helpers: unknown): AttachedFilesStore | undefined {
	return (helpers as AttachedFilesHelper | undefined)?.attachedFiles
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
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = searchFilesSchema.parse(args)
		const ready = store.readyFiles()
		if (ready.length === 0) {
			return 'Attached files are still being indexed. Try again shortly.'
		}
		toolCallbacks.setToolStatus(toolId, {
			content: `Searching attached files for /${parsed.pattern}/...`
		})

		const result = await searchFiles(ready, parsed.pattern, {
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
	'Read a bounded window of lines from a user-attached file. Returns line-numbered context with a pagination note. Files are not in context, so use this to inspect their contents.'
)

export const readFileTool: Tool<{}> = {
	def: readFileToolDef,
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = readFileSchema.parse(args)
		const entry = store.get(parsed.file)
		if (!entry || entry.status !== 'ready') {
			if (entry?.status === 'indexing') {
				return `File "${parsed.file}" is still being indexed. Try again shortly.`
			}
			if (entry?.status === 'error') {
				return `File "${parsed.file}" failed to load: ${entry.error ?? 'unknown error'}.`
			}
			const names = store
				.list()
				.map((f) => f.name)
				.join(', ')
			return `No attached file named "${parsed.file}". Attached files: ${names || '(none)'}.`
		}
		toolCallbacks.setToolStatus(toolId, { content: `Reading "${parsed.file}"...` })

		try {
			const res = await readFile(entry, {
				startLine: parsed.start_line,
				endLine: parsed.end_line
			})
			return res.text ? `${res.note}\n\n${res.text}` : res.note
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
	if (f.status === 'error') return `- ${f.name} (failed to load)`
	return `- ${f.name} — ${f.lineCount} lines, ${humanSize(f.size)}`
}

/** Build the `## Attached files` system-prompt section (metadata only, never content). */
export function buildAttachedFilesRoster(files: AttachedFile[]): string {
	if (files.length === 0) return ''
	const lines = files.map(rosterLine).join('\n')
	return [
		'## Attached files',
		'The user has attached the following files to this conversation. Their contents are NOT included here.',
		'Use the `search_files` tool to find content with a regex, and `read_file` to read a bounded window of lines.',
		'',
		lines
	].join('\n')
}

/**
 * Return a copy of the system message with the attached-files roster appended.
 * Always derives from the provided base so the roster never accumulates across turns.
 */
export function appendAttachedFilesRoster(
	base: ChatCompletionSystemMessageParam,
	files: AttachedFile[]
): ChatCompletionSystemMessageParam {
	const roster = buildAttachedFilesRoster(files)
	if (!roster || typeof base.content !== 'string') return base
	return { ...base, content: `${base.content}\n\n${roster}` }
}
