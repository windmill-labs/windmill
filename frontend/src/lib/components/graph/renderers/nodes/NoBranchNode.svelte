<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import { computeBorderStatus } from '../utils'

	interface Props {
		data: {
			id: string
			insertable: boolean
			eventHandlers: GraphEventHandlers
			flowModuleStates: Record<string, GraphModuleState> | undefined
			offset: number
			label: string | undefined
			branchOne: boolean
		}
	}

	let { data }: Props = $props()

	let borderStatus = $derived(
		data.branchOne
			? computeBorderStatus(0, 'branchone', data.flowModuleStates?.[data.id])
			: undefined
	)
</script>

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label ?? 'No branches'}
			id={data.id}
			hideId={true}
			selectable={true}
			selected={false}
			bgColor={getStateColor(undefined, darkMode)}
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			borderColor={getStateColor(borderStatus, darkMode)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/snippet}
</NodeWrapper>
