<script lang="ts">
	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import type { WorkspaceItem, WorkspaceItemKind } from '$lib/components/workspacePicker'
	import type { SessionTarget } from './sessionState.svelte'

	// `pipeline` targets never hit this component (they aren't slot-loaded, so they
	// can't 404 through SessionEditorTarget) — exclude it from the kinds here.
	type NotFoundKind = Exclude<SessionTarget['kind'], 'pipeline'>

	const KIND_NOT_FOUND_LABEL: Record<NotFoundKind, string> = {
		flow: 'Flow',
		script: 'Script',
		raw_app: 'Raw app'
	}

	let {
		kind,
		path,
		onNavigate
	}: {
		kind: NotFoundKind
		path: string
		onNavigate?: (item: WorkspaceItem) => void
	} = $props()

	// EditorHeader's `kind` prop is 'flow' | 'script' | 'app'; raw apps are
	// flagged via the `raw_app` boolean so the picker routes to /apps_raw/...
	const headerKind: WorkspaceItemKind = $derived(kind === 'raw_app' ? 'app' : kind)
	const isRawApp = $derived(kind === 'raw_app')

	// Local read-only mirrors so EditorHeader's bind:* doesn't write back into
	// the parent's session state. The pen popover is disabled below so these
	// stay quiet — but bindable references are still required by the API.
	let summary = $state('')
	let displayPath = $state(path)
	$effect(() => {
		displayPath = path
	})
</script>

<div class="flex flex-col h-full">
	<div class="flex h-12 items-center px-4 border-b border-border-light">
		<EditorHeader
			bind:summary
			bind:path={displayPath}
			savedPath={path}
			kind={headerKind}
			raw_app={isRawApp}
			summaryEditable={false}
			pathEditable={false}
			{onNavigate}
		/>
	</div>
	<div class="p-4 text-secondary text-sm">
		{KIND_NOT_FOUND_LABEL[kind]} not found at path
		<code class="font-mono">{path}</code>.
	</div>
</div>
