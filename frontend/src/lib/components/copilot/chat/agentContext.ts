import type { Tool } from './shared'
import type { AttachedFileStatus } from './files/attachedFiles.svelte'

/** One tool as the settings modal lists it — the model-facing name, description and
 * argument schema, which is exactly what the tool definition sends. */
export type ToolSummary = {
	name: string
	description: string
	/** The JSON Schema of the tool's arguments. `required` is defaulted because
	 * `SchemaViewer` reads it to mark the rows and renders nothing without it, and a
	 * tool whose arguments are all optional legitimately omits it. */
	parameters: Record<string, any>
}

/** Tool definitions as the modal lists them: name-sorted, so a list of dozens is
 * scannable and stays put as the set changes between turns. */
export function summarizeTools(tools: readonly Tool<any>[]): ToolSummary[] {
	return tools
		.map((t) => ({
			name: t.def.function.name,
			description: t.def.function.description ?? '',
			parameters: { required: [], ...(t.def.function.parameters ?? {}) }
		}))
		.sort((a, b) => a.name.localeCompare(b.name))
}

/** Why an attachment is not reachable, or undefined when it is. The file tools operate
 * on `readyFiles()`, so every other status is attached-but-unreadable and has to say so
 * — a row that looks like the readable ones is the one place this could claim something
 * the assistant cannot actually open. */
export function attachmentStatusLabel(status: AttachedFileStatus): string | undefined {
	switch (status) {
		case 'ready':
			return undefined
		case 'locked':
			return 'needs access'
		case 'unavailable':
			return 'unavailable'
		case 'indexing':
			return 'indexing…'
		case 'error':
			return 'failed'
	}
}

/** How many attachments the assistant can actually read, mirroring `readyFiles()`.
 *
 * A folder is counted on its children, never on its own status: that status is an
 * aggregate, so one indexing child would hide the readable rest, while an empty or
 * all-binary folder keeps a `ready` placeholder that `readyFiles()` filters out and
 * reads as usable while exposing nothing. */
export function countReadyAttachments(
	folders: readonly { files: readonly { status: AttachedFileStatus }[] }[],
	files: readonly { status: AttachedFileStatus }[]
): number {
	const isReady = (f: { status: AttachedFileStatus }) => f.status === 'ready'
	return folders.filter((d) => d.files.some(isReady)).length + files.filter(isReady).length
}
