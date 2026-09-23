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
 * Deliveries are keyed by tool call id rather than marked in the result text, where
 * a script quoting the marker would pass for a delivery that never happened, or on
 * the message object, which the completions path sends to the provider verbatim. A
 * delivery counts only while its tool message is still in the conversation, so
 * compaction or a restored chat gets the instructions again. */
export type FolderInstructionsContext = {
	list: () => readonly FolderInstruction[]
	deliveredBy: Map<string, readonly string[]>
}

function deliveredIn(ctx: FolderInstructionsContext, messages: readonly unknown[]): Set<string> {
	const delivered = new Set<string>()
	for (const m of messages) {
		const id = (m as { role?: unknown; tool_call_id?: unknown } | undefined)?.tool_call_id
		if (typeof id !== 'string') continue
		for (const p of ctx.deliveredBy.get(id) ?? []) delivered.add(p)
	}
	return delivered
}

/** The not-yet-delivered instructions covering the paths a tool call names, as the
 * text to hand the model, or undefined when there is nothing new. A body that no
 * longer reads (deleted or moved since the listing) is left out rather than failing
 * the call it rides on. */
export async function pendingFolderInstructions(
	ctx: FolderInstructionsContext | undefined,
	messages: readonly unknown[],
	args: unknown,
	workspace: string
): Promise<{ paths: string[]; text: string } | undefined> {
	if (!ctx || !workspace) return undefined
	const paths = workspacePathsInArgs(args)
	if (paths.length === 0) return undefined
	const delivered = deliveredIn(ctx, messages)
	const pending = instructionsCovering(ctx.list(), paths).filter((i) => !delivered.has(i.path))
	if (pending.length === 0) return undefined
	const blocks = (
		await Promise.all(
			pending.map(async (instruction) => {
				try {
					return { instruction, body: await readFolderInstructionBody(workspace, instruction.path) }
				} catch (e) {
					console.error(`Failed to read folder instructions ${instruction.path}`, e)
					return undefined
				}
			})
		)
	).filter((b) => b !== undefined)
	if (blocks.length === 0) return undefined
	return {
		paths: blocks.map((b) => b.instruction.path),
		text: formatFolderInstructions(blocks)
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

export function formatFolderInstructions(
	blocks: readonly { instruction: FolderInstruction; body: string }[]
): string {
	return blocks
		.map(
			({ instruction, body }) =>
				`<folder_instructions path="${instruction.path}" scope="${instruction.scope}">\n${truncateForPrompt(body, MAX_FOLDER_INSTRUCTIONS_LENGTH)}\n</folder_instructions>`
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
