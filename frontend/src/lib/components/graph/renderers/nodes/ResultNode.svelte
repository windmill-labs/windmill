<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { Writable } from 'svelte/store'
	import { getContext } from 'svelte'
	import type { ResultN } from '../../graphBuilder.svelte'

	interface Props {
		data: ResultN['data']
	}

	let { data }: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper enableSourceHandle={false}>
	{#snippet children({ darkMode })}
		<VirtualItem
			id={'Result'}
			label={'Result'}
			selectable={true}
			selected={$selectedId === 'Result'}
			hideId={true}
			bgColor={getStateColor(
				data.success == undefined ? undefined : data.success ? 'Success' : 'Failure',
				darkMode
			)}
			bgHoverColor={getStateHoverColor(
				data.success == undefined ? undefined : data.success ? 'Success' : 'Failure',
				darkMode
			)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
