// Shared state for the "open resource" modal triggered by clicking a Codex-style
// preview card under a tool call (see ToolExecutionDisplay.svelte). The modal itself
// is mounted at the page level (e.g. /design-sandbox) so it overlays the whole UI.

export type PreviewModalKind = 'schedule' | 'flow' | 'script' | 'app'

type State = {
	kind: PreviewModalKind | null
	path: string
	title: string
	isFlow: boolean
}

export const previewModalState = $state<State>({
	kind: null,
	path: '',
	title: '',
	isFlow: false
})

export function openPreviewModal(opts: {
	kind: PreviewModalKind
	path?: string
	title?: string
	isFlow?: boolean
}) {
	previewModalState.kind = opts.kind
	previewModalState.path = opts.path ?? ''
	previewModalState.title = opts.title ?? ''
	previewModalState.isFlow = opts.isFlow ?? false
}

export function closePreviewModal() {
	previewModalState.kind = null
}
