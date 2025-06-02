<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { BranchAllEndN } from '../../graphBuilder.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'

	interface Props {
		data: BranchAllEndN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={'Collect result from all branches'}
			id={data.id}
			selectable={true}
			selected={false}
			bgColor={getStateColor(undefined, darkMode)}
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			borderColor={getStateColor(data?.flowModuleStates?.[data?.id]?.type, darkMode)}
			on:select={(e) => {
				data?.eventHandlers?.select(e.detail)
			}}
		/>
	{/snippet}
</NodeWrapper>
