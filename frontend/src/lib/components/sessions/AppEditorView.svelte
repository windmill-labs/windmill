<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
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
			untrack(() => runtime.loadApp(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadApp(workspaceId, path)
	}
</script>

<DiffDrawer
	bind:this={diffDrawer}
	restoreDeployed={restoreFromCurrentTarget}
	restoreDraft={restoreFromCurrentTarget}
/>
{#if runtime.loadingApp && !runtime.loadedAppPath}
	<div class="p-4 text-secondary text-sm">Loading app {path}…</div>
{:else if runtime.notFoundApp && !runtime.loadedAppPath}
	<div class="p-4 text-secondary text-sm">App not found at path {path}</div>
{:else if runtime.appStore.val}
	<AppEditor
		summary={runtime.appStore.val.summary ?? ''}
		app={runtime.appStore.val.value}
		newPath={runtime.appStore.val.path}
		{path}
		policy={runtime.appStore.val.policy}
		bind:savedApp={runtime.savedApp.val}
		version={runtime.appStore.val.versions
			? runtime.appStore.val.versions[runtime.appStore.val.versions.length - 1]
			: undefined}
		newApp={false}
		replaceStateFn={() => {}}
		gotoFn={() => {}}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
