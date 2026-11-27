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
import { createToolDef } from '../shared'
import { NONE, type SessionTool } from '../sessionCapabilities'
import {
	readFile,
	searchFilesInWorker,
	numberLines,
	FileReadError,
	type SearchHit
} from './fileEngine'
import type { AttachedFile, AttachedFilesStore } from './attachedFiles.svelte'
import { sanitizeAttachmentName } from '../textFileUtils'
import { isViewableImageName } from './imageFiles'
import type {
	ImageResultFormat,
	ImageSource,
	ResultPath
} from '$lib/components/sessions/previewRouter'
import { JobService } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'
import { isS3ImageKey, parseS3Object, s3ImagePreviewUrl } from '$lib/utils'

/** Slice of the GLOBAL tool helpers that exposes the attached-files store. */
export interface AttachedFilesHelper {
	attachedFiles?: AttachedFilesStore
	/** Session chats only: shows an image in the side-panel preview. */
	openImage?: (source: ImageSource, label: string) => void
	/** The session's own (possibly forked) workspace, where its jobs run. */
	operatingWorkspace?: string
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
function notReadyMessage(
	store: AttachedFilesStore,
	file: string,
	helpers: unknown
): string | undefined {
	const entry = store.resolve(file)
	if (entry?.status === 'ready') {
		if (!entry.image) return undefined
		// show_image exists only where an image can open: a session's preview panel.
		return (helpers as AttachedFilesHelper | undefined)?.openImage
			? `"${file}" is an image, which has no text to read or search. Show it to the user with show_image.`
			: `"${file}" is an image, which has no text to read or search.`
	}
	if (entry?.status === 'indexing')
		return `File "${file}" is still being indexed. Try again shortly.`
	if (entry?.status === 'locked')
		return `File "${file}" is locked after a reload. Ask the user to restore access (send a message, or click "Restore access").`
	if (entry?.status === 'unavailable')
		return `File "${file}" is no longer available (moved, deleted, or its local copy was evicted). Ask the user to re-link it.`
	if (entry?.status === 'error')
		return `File "${file}" failed to load: ${entry.error ?? 'unknown error'}.`
	// Re-sanitized: folder-root placeholder rows carry the raw folder key.
	const names = store
		.list()
		.map((f) =>
			f.id ? `${sanitizeAttachmentName(f.name)} (file id: ${f.id})` : sanitizeAttachmentName(f.name)
		)
		.join(', ')
	return `No attached file matching "${file}". Attached files: ${names || '(none)'}.`
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
			'Optional file to restrict the search to: its file id when one is listed, otherwise its exact filename. Omit to search across all attached files.'
		),
	ignore_case: z.boolean().optional().describe('Case-insensitive matching. Defaults to false.')
})

const searchFilesToolDef = createToolDef(
	searchFilesSchema,
	'search_files',
	'Search the user-attached files with a regular expression and return matching lines with their line numbers. Use this to locate content before reading a specific window with read_file.'
)

export const searchFilesTool: SessionTool<{}> = {
	requires: NONE,
	def: searchFilesToolDef,
	planModeSafe: true,
	lookup: true,
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = searchFilesSchema.parse(args)
		// Validate a specifically requested file against the full store first, so a non-ready
		// target reports its real status (indexing/locked/…) instead of "not attached".
		if (parsed.file) {
			const notReady = notReadyMessage(store, parsed.file, helpers)
			if (notReady) return notReady
		}
		// Restrict to the resolved row itself, not a name filter: display names may
		// collide (same-named attachments on different messages), and a name filter
		// would silently search all of them under one label.
		const target = parsed.file ? store.resolve(parsed.file) : undefined
		const ready = target ? [target] : store.readyFiles()
		if (ready.length === 0) {
			return noReadyFilesMessage(store)
		}
		toolCallbacks.setToolStatus(toolId, {
			content: `Searching attached files for /${parsed.pattern}/...`
		})

		// Hit lines are the model's only handle on which row matched, and display
		// names may collide — label id-bearing rows with the reference that
		// resolves back to exactly that row.
		const rows = ready.map((f) => (f.id ? { ...f, name: `${f.name} (file id: ${f.id})` } : f))
		// Run in a Worker so a pathological model-supplied regex can't freeze the tab.
		const result = await searchFilesInWorker(rows, parsed.pattern, {
			flags: parsed.ignore_case ? 'i' : ''
		})
		if (result.error) {
			return `Error: ${result.error}`
		}
		const scope = target ? `"${target.name}"` : `${ready.length} file(s)`
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
	file: z
		.string()
		.describe('File to read: its file id when one is listed, otherwise its exact filename.'),
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

export const readFileTool: SessionTool<{}> = {
	requires: NONE,
	def: readFileToolDef,
	planModeSafe: true,
	lookup: true,
	fn: async ({ args, helpers, toolId, toolCallbacks }) => {
		const store = storeFrom(helpers)
		if (!store || store.count === 0) {
			return 'No files are attached to this conversation.'
		}
		const parsed = readFileSchema.parse(args)
		const notReady = notReadyMessage(store, parsed.file, helpers)
		if (notReady) return notReady
		const entry = store.resolve(parsed.file)!
		toolCallbacks.setToolStatus(toolId, { content: `Reading "${entry.name}"...` })

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

const showImageSchema = z.object({
	file: z
		.string()
		.optional()
		.describe('Exact filename of an attached image, as the roster lists it.'),
	job_id: z
		.string()
		.optional()
		.describe('A completed job whose result holds images (a run, a flow).'),
	step_id: z
		.string()
		.optional()
		.describe('With job_id on a flow: the step whose result holds the image, by its id.'),
	path: z
		.array(z.union([z.string(), z.number()]))
		.optional()
		.describe(
			'With job_id: keys and list indexes from the root of the result (of the step, with step_id) to the image, e.g. [2] or ["charts", 0, "png"]. Without it, every image the result holds is shown.'
		),
	s3: z
		.object({
			s3: z.string().describe('The object key, e.g. "charts/plot.png".'),
			storage: z
				.string()
				.optional()
				.describe('The secondary storage holding it, when not the workspace default.')
		})
		.optional()
		.describe('An S3 image object, as a result holds it: `{ s3, storage? }`.')
})

const showImageToolDef = createToolDef(
	showImageSchema,
	'show_image',
	'Show an image to the user in the side-panel preview next to the chat, where they can zoom and pan it. Pass exactly one of: `file` for an image the user attached (png, jpeg, gif, webp, avif, bmp, ico or svg); `job_id` (and `step_id` for a flow step) for a job result, showing every image it holds (inline png, jpeg or gif, S3 image objects, each `render_all` item), or with `path` only the image at that place in the result, read with get_run first; or `s3` for an S3 image object. Call it again to show more images. You cannot see the image yourself: this only displays it to the user.'
)

const RESULT_IMAGE_FORMATS: ImageResultFormat[] = ['png', 'jpeg', 'gif']

export type ResultImage =
	| { format: ImageResultFormat; path: ResultPath }
	| { s3: { s3: string; storage?: string } }

function s3Image(value: unknown): ResultImage | undefined {
	if (typeof value === 'string') {
		if (!/^s3:\/\/[^/]*\/.+/.test(value)) return undefined
	} else if (typeof (value as { s3?: unknown } | undefined)?.s3 !== 'string') return undefined
	const s3 = parseS3Object(value as Parameters<typeof parseS3Object>[0])
	return isS3ImageKey(s3.s3) ? { s3: { s3: s3.s3, storage: s3.storage } } : undefined
}

/** The images a result holds, where DisplayResult renders them: the result's own
 * `png`/`jpeg`/`gif` key (a base64 string or `{ content }`), an S3 image object, the images
 * among a list of S3 objects (DisplayResult lists any mix of file types), and each
 * `render_all` item. */
export function resultImages(value: unknown, path: ResultPath = []): ResultImage[] {
	const s3 = s3Image(value)
	if (s3) return [s3]
	if (Array.isArray(value)) {
		const isS3List = value.every((i) => typeof (i as { s3?: unknown } | null)?.s3 === 'string')
		return isS3List ? value.flatMap((i) => s3Image(i) ?? []) : []
	}
	if (!value || typeof value !== 'object') return []
	const obj = value as Record<string, unknown>
	if (Array.isArray(obj.render_all)) {
		return obj.render_all.flatMap((child, i) => resultImages(child, [...path, 'render_all', i]))
	}
	const format = RESULT_IMAGE_FORMATS.find((f) => {
		const v = obj[f]
		return typeof v === 'string' || typeof (v as { content?: unknown })?.content === 'string'
	})
	return format ? [{ format, path }] : []
}

/** The images at `path` inside a result. A path may end on the image's base64 field
 * (`[0, "png"]`), which names the same image as the object holding it. */
export function imagesAtPath(result: unknown, path: ResultPath): ResultImage[] {
	const last = path.at(-1)
	const format = RESULT_IMAGE_FORMATS.find((f) => f === last)
	const at = format ? path.slice(0, -1) : path
	let value = result
	for (const key of at) {
		if (!value || typeof value !== 'object') return []
		value = (value as Record<string | number, unknown>)[key]
	}
	if (format) {
		// The named field, not the first image field of the object holding it.
		const v = (value as Record<string, unknown> | undefined)?.[format]
		const isImage =
			typeof v === 'string' || typeof (v as { content?: unknown })?.content === 'string'
		return isImage ? [{ format, path: at }] : []
	}
	return resultImages(value, at)
}

async function showResultImages(
	openImage: (source: ImageSource, label: string) => void,
	workspace: string,
	jobId: string,
	stepId: string | undefined,
	path: ResultPath | undefined
): Promise<string> {
	const what = `job ${jobId}${stepId ? ` step ${stepId}` : ''}`
	let result: unknown
	try {
		result = stepId
			? await JobService.resultById({ workspace, flowJobId: jobId, nodeId: stepId })
			: await JobService.getCompletedJobResult({ workspace, id: jobId })
	} catch (e) {
		return `Could not load the result of ${what}: ${e instanceof Error ? e.message : String(e)}`
	}
	const images = path?.length ? imagesAtPath(result, path) : resultImages(result)
	if (images.length === 0) {
		return path?.length
			? `Nothing at ${JSON.stringify(path)} in the result of ${what} is an image. Read the result with get_run to find the image's path.`
			: `The result of ${what} holds no image.`
	}
	const base = stepId ? `${stepId} result` : `Result ${jobId.slice(-8)}`
	for (const image of images) {
		if ('s3' in image) {
			openImage(
				{ kind: 'url', url: s3ImagePreviewUrl(workspace, image.s3) },
				image.s3.s3.split('/').pop() || image.s3.s3
			)
			continue
		}
		const { format, path } = image
		const at = path.length ? ` #${path.filter((k) => typeof k === 'number').join('.')}` : ''
		openImage(
			{
				kind: 'result',
				workspace,
				jobId,
				format,
				...(stepId ? { nodeId: stepId } : {}),
				...(path.length ? { path } : {})
			},
			base + at
		)
	}
	return images.length === 1
		? 'Showing the result image in the preview panel.'
		: `Showing the ${images.length} result images in the preview panel, one tab each.`
}

export const showImageTool: SessionTool<{}> = {
	requires: NONE,
	def: showImageToolDef,
	planModeSafe: true,
	fn: async ({ args, helpers }) => {
		const h = helpers as AttachedFilesHelper | undefined
		const openImage = h?.openImage
		if (!openImage) return 'Images can only be shown inside an AI session.'
		const parsed = showImageSchema.parse(args)
		const given = [parsed.file, parsed.job_id, parsed.s3].filter((v) => v !== undefined).length
		if (given !== 1) {
			return 'Pass exactly one of `file` (an attached image), `job_id` (a job result) or `s3` (an S3 image object).'
		}
		if (!parsed.job_id && (parsed.step_id || parsed.path)) {
			return '`step_id` and `path` locate an image in a job result: pass them with `job_id`.'
		}
		if (parsed.job_id || parsed.s3) {
			const workspace = h?.operatingWorkspace ?? get(workspaceStore)
			if (!workspace) return 'No workspace to load the image from.'
			if (parsed.job_id) {
				return showResultImages(openImage, workspace, parsed.job_id, parsed.step_id, parsed.path)
			}
			const s3 = parsed.s3!
			if (!isS3ImageKey(s3.s3)) {
				return `"${s3.s3}" is not an image the viewer can show (png, jpeg or webp).`
			}
			openImage(
				{ kind: 'url', url: s3ImagePreviewUrl(workspace, s3) },
				s3.s3.split('/').pop() || s3.s3
			)
			return `Showing "${s3.s3}" in the preview panel.`
		}
		if (!parsed.file) return 'Pass `file` for an attached image or `job_id` for a job result.'
		const store = storeFrom(helpers)
		if (!store) return 'No files are attached to this chat.'
		const entry = store.resolve(parsed.file)
		if (!entry || entry.messageScoped || entry.isFolderRoot) {
			return (
				notReadyMessage(store, parsed.file, helpers) ??
				`No attached file matching "${parsed.file}".`
			)
		}
		if (!isViewableImageName(entry.sourceName ?? entry.name)) {
			return `"${entry.name}" is not an image. Read it with read_file instead.`
		}
		if (entry.status !== 'ready') {
			return notReadyMessage(store, parsed.file, helpers) ?? `"${entry.name}" is not available.`
		}
		openImage({ kind: 'file', name: entry.name }, entry.name.split('/').pop() || entry.name)
		return `Showing "${entry.name}" in the preview panel.`
	}
}

export const fileTools: SessionTool<{}>[] = [searchFilesTool, readFileTool, showImageTool]

function rosterLine(f: AttachedFile): string {
	// Message rows are addressed by their stable id (names may collide); session
	// rows by name. Names are sanitized at render: this block is model-facing
	// prompt text and stored names (legacy, folder children) may carry controls.
	const ref = f.id
		? `${sanitizeAttachmentName(f.name)} (file id: ${f.id})`
		: sanitizeAttachmentName(f.name)
	if (f.status === 'indexing') return `- ${ref} (indexing…)`
	if (f.status === 'locked') return `- ${ref} (locked — needs the user to restore access)`
	if (f.status === 'unavailable') return `- ${ref} (unavailable)`
	if (f.status === 'error') return `- ${ref} (failed to load)`
	if (f.image) return `- ${ref} — image, ${humanSize(f.size)}`
	return `- ${ref} — ${f.lineCount} lines, ${humanSize(f.size)}`
}

/** Build the `## Attached files` system-prompt section (metadata only, never content). */
export function buildAttachedFilesRoster(
	store: AttachedFilesStore,
	orphanedMessageFileIds?: Set<string>
): string {
	const lines: string[] = []
	for (const folder of store.folders) {
		// Folder names are RAW disk keys (never sanitized in the store — they must
		// match the handle, children, and persistence record), so this model-facing
		// render is where control characters get stripped.
		const folderName = sanitizeAttachmentName(folder.name)
		// A locked/unavailable folder has no readable children — one line for the whole folder.
		if (folder.status === 'locked') {
			lines.push(`- ${folderName} (locked — needs the user to restore access)`)
		} else if (folder.status === 'unavailable') {
			lines.push(`- ${folderName} (unavailable)`)
		} else {
			lines.push(...folder.files.map(rosterLine))
		}
	}
	// Message-attached files are deliberately NOT listed here: their reference
	// lives inside the message that carried them (or the compaction summary),
	// exactly like DOM picks — the roster only advertises session-wide links.
	lines.push(...store.standalone.map(rosterLine))
	// Exception: a message whose API counterpart was dropped by drop-oldest
	// compaction takes its in-message reference with it, so those attachments must
	// be advertised here or the model can no longer see they exist.
	if (orphanedMessageFileIds?.size) {
		lines.push(
			...store.messageAttached
				.filter((f) => f.id && orphanedMessageFileIds.has(f.id))
				.map(rosterLine)
		)
	}
	if (lines.length === 0) return ''
	return [
		'## Attached files',
		'The user has attached the following files to this conversation. Their contents are NOT included here.',
		'Use the `search_files` tool to find content with a regex, and `read_file` to read a bounded window of lines.',
		...([...store.folders.flatMap((d) => d.files), ...store.standalone].some((f) => f.image)
			? ['Images have no text: `show_image` displays one to the user, where it is available.']
			: []),
		'Reference a file by its file id when one is shown, otherwise by its filename.',
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
	store: AttachedFilesStore,
	orphanedMessageFileIds?: Set<string>
): ChatCompletionSystemMessageParam {
	const roster = buildAttachedFilesRoster(store, orphanedMessageFileIds)
	if (!roster || typeof base.content !== 'string') return base
	return { ...base, content: `${base.content}\n\n${roster}` }
}
