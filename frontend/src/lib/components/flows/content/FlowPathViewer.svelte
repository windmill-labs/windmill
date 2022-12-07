<script lang="ts">
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let path: string

	let flow: Flow | undefined = undefined
	async function loadFlow(path: string) {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
	}

	$: path && loadFlow(path)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto p-2">
	{#if flow}
		<FlowViewer {flow} />
	{:else}
		<Skeleton layout={[[40]]} />
	{/if}
</div>
