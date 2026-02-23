<script lang="ts">
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		summary?: string
		selected?: boolean
	}

	let { summary, selected = false }: Props = $props()

	let colorClasses = $derived(getNodeColorClasses(undefined, selected))
</script>

<div
	class={twMerge(
		'w-full module flex rounded-md cursor-pointer max-w-full drop-shadow-base',
		colorClasses.bg
	)}
	style="width: 275px; height: 34px;"
>
	<div
		class={twMerge('absolute z-0 rounded-md outline-offset-0', colorClasses.outline)}
		style="width: 275px; height: 34px;"
	></div>
	<div class="flex flex-col w-full">
		<FlowModuleSchemaItemViewer label={summary || 'Group'} {colorClasses}>
			{#snippet icon()}
				<BarsStaggered size={16} />
			{/snippet}
		</FlowModuleSchemaItemViewer>
	</div>
</div>
