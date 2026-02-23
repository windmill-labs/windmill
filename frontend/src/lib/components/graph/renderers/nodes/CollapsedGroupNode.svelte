<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Maximize2 } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()

	let selected = $derived(!!(selectionManager && selectionManager.isNodeSelected(id)))
	let colorClasses = $derived(getNodeColorClasses(undefined, selected))
	let hover = $state(false)
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class={twMerge(
				'w-full module flex rounded-md cursor-pointer max-w-full drop-shadow-base',
				colorClasses.bg
			)}
			style="width: 275px; height: 34px;"
			onmouseenter={() => (hover = true)}
			onmouseleave={() => (hover = false)}
		>
			<div
				class={twMerge('absolute z-0 rounded-md outline-offset-0', colorClasses.outline)}
				style="width: 275px; height: 34px;"
			></div>
			<div class="flex flex-col w-full">
				<FlowModuleSchemaItemViewer
					label={data.summary || 'Group'}
					{colorClasses}
				>
					{#snippet icon()}
						<BarsStaggered size={16} />
					{/snippet}
				</FlowModuleSchemaItemViewer>
			</div>

			<div class="absolute -translate-y-[100%] top-2 right-10 h-7 p-1">
				<button
					title="Expand group"
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
			</div>
		</div>
	{/snippet}
</NodeWrapper>
