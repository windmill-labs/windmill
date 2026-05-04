<script lang="ts">
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
	} = $props()

	let selectedId = $state('settings-metadata')
	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadFlow(workspaceId, path))
		}
	})

	// In a session pane, "restore" just reloads from the current state — the
	// session target stays put. The Diff drawer's primary use here is viewing
	// the diff; restore is best-effort.
	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadFlow(workspaceId, path)
	}
</script>

<DiffDrawer
	bind:this={diffDrawer}
	restoreDeployed={restoreFromCurrentTarget}
	restoreDraft={restoreFromCurrentTarget}
	isFlow
/>
{#if runtime.loadingFlow && !runtime.loadedPath}
	<div class="p-4 text-secondary text-sm">Loading flow {path}…</div>
{:else if runtime.notFound && !runtime.loadedPath}
	<div class="p-4 text-secondary text-sm">Flow not found at path {path}</div>
{:else}
	<FlowBuilder
		flowStore={runtime.flowStore}
		flowStateStore={runtime.flowStateStore}
		initialPath={path}
		newFlow={false}
		{selectedId}
		loading={runtime.loadingFlow && !runtime.loadedPath}
		bind:savedFlow={runtime.savedFlow.val}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
