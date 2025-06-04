<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import { computeBorderStatus } from '../utils'
	import type { BranchOneStartN } from '../../graphBuilder.svelte'

	interface Props {
		data: BranchOneStartN['data']
	}

	let { data }: Props = $props()

	let borderStatus = $derived(
		computeBorderStatus(data.branchIndex + 1, 'branchone', data.flowModuleStates?.[data.id])
	)
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
			borderColor={borderStatus ? getStateColor(borderStatus, darkMode) : undefined}
			on:select={() => {
				setTimeout(() => data?.eventHandlers?.select(data.id))
			}}
		/>
		{#if data.insertable}
			<button
				title="Delete branch"
				class="z-50 absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] center-center text-primary
	border-[1.5px] border-gray-700 bg-surface duration-0 hover:bg-red-400 hover:text-white
	hover:border-red-700"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.deleteBranch(
							{
								id: data.id,
								index: data.branchIndex + 1
							},
							data.label
						)
					})
				)}
			>
				<X size={12} />
			</button>
		{/if}
	{/snippet}
</NodeWrapper>
