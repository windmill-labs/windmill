<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Minimize2 } from 'lucide-svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { SubflowBoundN } from '../../graphBuilder.svelte'

	interface Props {
		data: SubflowBoundN['data']
	}

	let { data }: Props = $props()
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<VirtualItem
			label={data.label}
			preLabel={data.preLabel}
			selectable
			selected={data.selected}
			bgColor={getStateColor(undefined, darkMode)}
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			borderColor={undefined}
			on:select={() => {
				setTimeout(() => data.eventHandlers?.select(data.id))
			}}
		/>
		<button
			title="Unexpand subflow"
			class="z-50 absolute -top-[10px] right-[25px] rounded-full h-[20px] w-[20px] center-center text-primary bg-surface duration-0 hover:bg-surface-hover"
			onclick={stopPropagation(
				preventDefault(() => {
					data.eventHandlers.minimizeSubflow(data.subflowId)
				})
			)}
		>
			<Minimize2 size={12} />
		</button>
	{/snippet}
</NodeWrapper>
