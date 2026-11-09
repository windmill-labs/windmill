// No imports: the CLI's review bundle carries this module (see cli/generate-ai-review.ts).

/** Bytes of one instruction body a tool result may carry. */
export const MAX_FOLDER_INSTRUCTIONS_LENGTH = 32 * 1024
/** Bytes all the bodies one tool result carries may add up to, so nested folders
 * cannot stack several full-size bodies onto a single result. */
export const MAX_FOLDER_INSTRUCTIONS_TOTAL = 48 * 1024

export type FolderInstruction = {
	/** The instruction resource. */
	path: string
	/** The directory it covers, with a trailing `/`: `f/billing/AGENTS` covers `f/billing/`. */
	scope: string
}

export function folderInstructionScope(path: string): string {
	return path.slice(0, path.lastIndexOf('/') + 1)
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

/** Cut `text` to `maxBytes` of UTF-8, marking the cut so a reader (the model
 * included) can tell truncation from a body that simply ends there.
 *
 * For the body, whose cap is a byte budget: 64k CJK characters are ~192 KiB, so a
 * code-unit cut would let three times the intended payload through. */
export function truncateForPrompt(text: string, maxBytes: number): string {
	const encoded = new TextEncoder().encode(text)
	if (encoded.byteLength <= maxBytes) return text
	// `fatal: false` replaces the partial code point a byte-aligned cut can leave
	// with U+FFFD; dropping it keeps the tail clean.
	const cut = new TextDecoder('utf-8').decode(encoded.slice(0, maxBytes)).replace(/�$/, '')
	return `${cut}… [truncated]`
}

export function folderInstructionsOpeningTag(path: string): string {
	return `<folder_instructions path="${path}"`
}

export function formatFolderInstructions(
	blocks: readonly { instruction: FolderInstruction; body: string }[]
): string {
	// The budget goes innermost first: the most specific instructions are the last to
	// be cut. An outer body the budget no longer reaches keeps its tag, so it still
	// counts as delivered, with the cut marked.
	let remaining = MAX_FOLDER_INSTRUCTIONS_TOTAL
	const bodies = [...blocks].reverse().map(({ body }) => {
		const cut = truncateForPrompt(body, Math.min(MAX_FOLDER_INSTRUCTIONS_LENGTH, remaining))
		remaining = Math.max(0, remaining - new TextEncoder().encode(cut).byteLength)
		return cut
	})
	bodies.reverse()
	return blocks
		.map(
			({ instruction }, i) =>
				`${folderInstructionsOpeningTag(instruction.path)} scope="${instruction.scope}">\n${bodies[i]}\n</folder_instructions>`
		)
		.join('\n\n')
}
