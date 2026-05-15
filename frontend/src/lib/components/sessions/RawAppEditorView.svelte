<script lang="ts">
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
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

	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadRawApp(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadRawApp(workspaceId, path)
	}
</script>

<DiffDrawer
	bind:this={diffDrawer}
	restoreDeployed={restoreFromCurrentTarget}
	restoreDraft={restoreFromCurrentTarget}
/>
{#if runtime.loadingRawApp && !runtime.loadedRawAppPath}
	<div class="p-4 text-secondary text-sm">Loading raw app {path}…</div>
{:else if runtime.notFoundRawApp && !runtime.loadedRawAppPath}
	<div class="p-4 text-secondary text-sm">Raw app not found at path {path}</div>
{:else if runtime.rawApp.val}
	<RawAppEditor
		bind:files={runtime.rawApp.val.files}
		bind:runnables={runtime.rawApp.val.runnables}
		bind:data={runtime.rawApp.val.data}
		bind:summary={runtime.rawApp.val.summary}
		newPath={runtime.rawApp.val.path}
		{path}
		policy={runtime.rawApp.val.policy}
		bind:savedApp={runtime.savedRawApp.val}
		newApp={false}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
