<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
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
			untrack(() => runtime.loadScript(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadScript(workspaceId, path)
	}
</script>

<DiffDrawer
	bind:this={diffDrawer}
	restoreDeployed={restoreFromCurrentTarget}
	restoreDraft={restoreFromCurrentTarget}
/>
{#if runtime.loadingScript && !runtime.loadedScriptPath}
	<div class="p-4 text-secondary text-sm">Loading script {path}…</div>
{:else if runtime.notFoundScript && !runtime.loadedScriptPath}
	<div class="p-4 text-secondary text-sm">Script not found at path {path}</div>
{:else if runtime.scriptStore.val}
	<ScriptBuilder
		bind:script={runtime.scriptStore.val}
		bind:savedScript={runtime.savedScript.val}
		initialPath={path}
		fullyLoaded={!runtime.loadingScript}
		disableHistoryChange={true}
		replaceStateFn={() => {}}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
