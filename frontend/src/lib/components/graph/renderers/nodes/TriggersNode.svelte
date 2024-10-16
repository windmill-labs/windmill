<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import { type GraphEventHandlers } from '../../graphBuilder'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'

	export let data: {
		path: string
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
		eventHandlers: GraphEventHandlers
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper wrapperClass="shadow-none">
	<TriggersWrapper
		path={data.path}
		on:select={() => {
			data?.eventHandlers?.select('triggers')
		}}
		isFlow={true}
		selected={$selectedId == 'triggers'}
		newItem={data.newFlow}
	/>
</NodeWrapper>
