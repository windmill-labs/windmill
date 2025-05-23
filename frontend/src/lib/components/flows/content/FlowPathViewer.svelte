<script lang="ts">
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import { FlowService, type Flow, type TriggersCount } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'

	export let path: string
	export let noSide = false

	let flow: Flow | undefined = undefined

	const triggersCount = writable<TriggersCount | undefined>(undefined)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount: triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState: new Triggers()
	})

	async function loadFlow(path: string) {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		triggersCount.set(
			await FlowService.getTriggersCountOfFlow({ workspace: $workspaceStore!, path })
		)
	}

	$: path && loadFlow(path)
</script>

<div class="flex flex-col flex-1 h-full overflow-auto">
	{#if flow}
		<FlowGraphViewer triggerNode={true} {noSide} {flow} />
	{:else}
		<Skeleton layout={[[40]]} />
	{/if}
</div>
