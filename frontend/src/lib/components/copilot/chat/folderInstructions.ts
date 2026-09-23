import { ResourceService } from '$lib/gen'
import { truncateForPrompt } from './skills/skillResources'

/**
 * Folder instructions are resources of this type: a markdown file resource
 * (`format_extension = 'md'`) whose `value.content` the assistant follows for
 * every item under the directory the resource lives in, like an AGENTS.md.
 */
export const FOLDER_INSTRUCTIONS_RESOURCE_TYPE = 'ai_instruction'

/** Bytes of one instruction body a tool result may carry. */
export const MAX_FOLDER_INSTRUCTIONS_LENGTH = 32 * 1024

/** Scopes the system prompt names. Past this the list is cut, not the delivery:
 * a scope left out still has its instructions delivered on first touch. */
export const MAX_LISTED_FOLDER_INSTRUCTIONS = 50

export type FolderInstruction = {
	/** The `ai_instruction` resource. */
	path: string
	/** The directory it covers, with a trailing `/`: `f/billing/AGENTS` covers `f/billing/`. */
	scope: string
}

export function folderInstructionScope(path: string): string {
	return path.slice(0, path.lastIndexOf('/') + 1)
}

const PAGE_SIZE = 100
/** Guard against a paging bug looping forever, not a product cap. */
const MAX_PAGES = 20

/** Every folder instruction the user can read in the workspace. Read access is the
 * resource's own ACL, so a folder's instructions reach exactly who can see them. */
export async function listFolderInstructions(workspace: string): Promise<FolderInstruction[]> {
	if (!workspace) return []
	const rows: FolderInstruction[] = []
	for (let page = 1; page <= MAX_PAGES; page++) {
		const resources = await ResourceService.listResource({
			workspace,
			resourceType: FOLDER_INSTRUCTIONS_RESOURCE_TYPE,
			page,
			perPage: PAGE_SIZE
		})
		rows.push(...resources.map((r) => ({ path: r.path, scope: folderInstructionScope(r.path) })))
		if (resources.length < PAGE_SIZE) break
	}
	return rows
}

/** Workspace paths a tool call names: every top-level string argument called `path`
 * or `*_path` that is a `u/` or `f/` path. */
export function workspacePathsInArgs(args: unknown): string[] {
	if (!args || typeof args !== 'object' || Array.isArray(args)) return []
	return Object.entries(args as Record<string, unknown>)
		.filter(
			([key, value]) =>
				(key === 'path' || key.endsWith('_path')) &&
				typeof value === 'string' &&
				/^[uf]\/[^/]+/.test(value)
		)
		.map(([, value]) => value as string)
}

/** The instructions covering any of `paths`, outermost scope first, so a nested
 * folder's instructions come last and read as the more specific ones. A path naming
 * the folder itself (`f/billing`, as a pipeline's is) is covered too. */
export function instructionsCovering(
	instructions: readonly FolderInstruction[],
	paths: readonly string[]
): FolderInstruction[] {
	return instructions
		.filter((i) => paths.some((p) => `${p}/`.startsWith(i.scope)))
		.sort((a, b) => a.scope.length - b.scope.length || a.path.localeCompare(b.path))
}

/** What a chat hands the tool loop: the instructions in play, and which tool call
 * delivered which of them.
 *
 * A delivery counts only while its tool message is still in the conversation and
 * carries the text: the id is recorded because a script quoting the tag in a result
 * must not pass for a delivery, and nothing is marked on the message object because
 * the completions path sends that to the provider verbatim. So compaction or a
 * restored chat gets the instructions again. */
export type FolderInstructionsContext = {
	list: () => readonly FolderInstruction[]
	deliveredBy: Map<string, readonly string[]>
}

/** Instructions delivered by the tool results in `messages`, split by whether the
 * model has read them. Results after the latest assistant message answer the batch
 * still being processed: the model has not seen those yet, so a delivery there
 * must not let a change later in the same batch through. */
function deliveredIn(
	ctx: FolderInstructionsContext,
	messages: readonly unknown[]
): { read: Set<string>; thisBatch: Set<string> } {
	const read = new Set<string>()
	const thisBatch = new Set<string>()
	let lastAssistant = -1
	messages.forEach((m, i) => {
		if ((m as { role?: unknown } | undefined)?.role === 'assistant') lastAssistant = i
	})
	messages.forEach((m, i) => {
		const { tool_call_id: id, content } = (m ?? {}) as { tool_call_id?: unknown; content?: unknown }
		if (typeof id !== 'string' || typeof content !== 'string') return
		for (const p of ctx.deliveredBy.get(id) ?? []) {
			// The id alone is not enough: a call stopped mid-run is answered with a
			// placeholder under the same id, which carries no instructions.
			if (content.includes(openingTag(p))) (i < lastAssistant ? read : thisBatch).add(p)
		}
	})
	return { read, thisBatch }
}

/** The instructions covering the paths a tool call names that the model has not
 * read yet, as the text to hand it, or undefined when there are none. `paths` are
 * the bodies the text carries; one already carried earlier in the same batch is
 * pointed to rather than repeated. A body that no longer reads (deleted or moved
 * since the listing) is left out rather than failing the call it rides on. */
export async function pendingFolderInstructions(
	ctx: FolderInstructionsContext | undefined,
	messages: readonly unknown[],
	args: unknown,
	workspace: string
): Promise<{ paths: string[]; text: string } | undefined> {
	if (!ctx || !workspace) return undefined
	const paths = workspacePathsInArgs(args)
	if (paths.length === 0) return undefined
	const { read, thisBatch } = deliveredIn(ctx, messages)
	const pending = instructionsCovering(ctx.list(), paths).filter((i) => !read.has(i.path))
	const inBatch = pending.filter((i) => thisBatch.has(i.path))
	const blocks = (
		await Promise.all(
			pending
				.filter((i) => !thisBatch.has(i.path))
				.map(async (instruction) => {
					try {
						return {
							instruction,
							body: await readFolderInstructionBody(workspace, instruction.path)
						}
					} catch (e) {
						console.error(`Failed to read folder instructions ${instruction.path}`, e)
						return undefined
					}
				})
		)
	).filter((b) => b !== undefined)
	if (blocks.length === 0 && inBatch.length === 0) return undefined
	const pointers = inBatch.map(
		(i) =>
			`The instructions for \`${i.scope}\` (${i.path}) are in an earlier result of this same batch of tool calls.`
	)
	return {
		paths: blocks.map((b) => b.instruction.path),
		text: [formatFolderInstructions(blocks), ...pointers].filter(Boolean).join('\n\n')
	}
}

export async function readFolderInstructionBody(workspace: string, path: string): Promise<string> {
	const value = (await ResourceService.getResourceValue({ workspace, path })) as
		| { content?: unknown }
		| undefined
	if (typeof value?.content !== 'string') {
		throw new Error(`resource ${path} has no string "content"`)
	}
	return value.content
}

function openingTag(path: string): string {
	return `<folder_instructions path="${path}"`
}

export function formatFolderInstructions(
	blocks: readonly { instruction: FolderInstruction; body: string }[]
): string {
	return blocks
		.map(
			({ instruction, body }) =>
				`${openingTag(instruction.path)} scope="${instruction.scope}">\n${truncateForPrompt(body, MAX_FOLDER_INSTRUCTIONS_LENGTH)}\n</folder_instructions>`
		)
		.join('\n\n')
}

/** The system prompt section naming the scopes that carry instructions. */
export function getFolderInstructionsPromptSection(
	instructions: readonly FolderInstruction[]
): string {
	if (instructions.length === 0) return ''
	const scopes = [...new Set(instructions.map((i) => i.scope))].sort()
	const listed = scopes.slice(0, MAX_LISTED_FOLDER_INSTRUCTIONS)
	const more = scopes.length > listed.length ? ` (and ${scopes.length - listed.length} more)` : ''
	return `\n\nFOLDER INSTRUCTIONS:
These folders carry instructions (ai_instruction resources) for every item under them: ${listed.map((s) => `\`${s}\``).join(', ')}${more}.
- The first time a tool call names a path under one of them, its instructions are delivered in a <folder_instructions> block of the tool result. A call that would change something is held back until you have them: read them, then make the call again.
- Follow them for all work under that folder. Where they differ from the general guidance above, the folder's instructions win, and a nested folder's win over its parent's. They never override the user's explicit requests in this chat.`
}
