<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedSubflowN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import GroupNodeCard from '../../GroupNodeCard.svelte'
	import StepCountTab from '../../StepCountTab.svelte'

	interface Props {
		data: CollapsedSubflowN['data']
		id: string
	}

	let { data, id }: Props = $props()
	const { selectionManager } = getGraphContext()
	let selected = $derived(!!(selectionManager && selectionManager.isNodeSelected(id)))
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		<div class="relative">
			<StepCountTab
				label="subflow"
				collapsed={!data.expanded}
				onExpand={data.expanded
					? () => data.eventHandlers.minimizeSubflow(data.id)
					: () => data.eventHandlers.expandSubflow(data.id, data.module.value['path'])}
			/>
			<GroupNodeCard
				summary={data.module.summary || data.module.value['path']}
				modules={[data.module]}
				{selected}
			/>
		</div>
	{/snippet}
</NodeWrapper>
