<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Maximize2 } from 'lucide-svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import { twMerge } from 'tailwind-merge'
	import GroupNodeCard from '../../GroupNodeCard.svelte'
	import { Tooltip } from '$lib/components/meltComponents'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()

	let selected = $derived(!!(selectionManager && selectionManager.isNodeSelected(id)))
	let hover = $state(false)
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="relative"
			onmouseenter={() => (hover = true)}
			onmouseleave={() => (hover = false)}
		>
			<GroupNodeCard summary={data.summary} color={data.color} {selected} />

			<div class="absolute -translate-y-[100%] top-2 right-10 h-7 p-1">
				<Tooltip>
					<button
						class={twMerge(
							'center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1',
							'shadow-md rounded-md',
							hover || selected ? 'opacity-100' : 'opacity-50'
						)}
						onclick={stopPropagation(
							preventDefault(() => {
								data.eventHandlers.expandGroup(data.groupId)
							})
						)}
						onpointerdown={stopPropagation(preventDefault(() => {}))}
					>
						<Maximize2 size={12} />
					</button>
					<svelte:fragment slot="text">Expand group</svelte:fragment>
				</Tooltip>
			</div>
		</div>
	{/snippet}
</NodeWrapper>
