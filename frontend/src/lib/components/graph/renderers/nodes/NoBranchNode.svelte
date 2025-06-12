<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { NoBranchN } from '../../graphBuilder.svelte'

	interface Props {
		data: NoBranchN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	{#snippet children({ darkMode })}
		<VirtualItem
			label="No branches"
			id={data.id}
			hideId={true}
			selectable={true}
			selected={false}
			bgColor={getStateColor(undefined, darkMode)}
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			borderColor={getStateColor(undefined, darkMode)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
