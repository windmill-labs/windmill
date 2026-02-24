<script lang="ts">
	import FlowModuleSchemaItemViewer from '$lib/components/flows/map/FlowModuleSchemaItemViewer.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { getNodeColorClasses } from '$lib/components/graph'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

	interface Props {
		summary?: string
		selected?: boolean
		stepCount?: number
		fullWidth?: boolean
		actions?: Snippet
	}

	let { summary, selected = false, stepCount, fullWidth = false, actions }: Props = $props()

	let colorClasses = $derived(getNodeColorClasses(undefined, selected))
</script>

<div
	class={twMerge(
		'w-full module flex rounded-md cursor-pointer max-w-full drop-shadow-base',
		colorClasses.bg
	)}
	style={fullWidth ? 'height: 34px;' : 'width: 275px; height: 34px;'}
>
	<div
		class={twMerge('absolute z-0 rounded-md outline-offset-0 inset-0', colorClasses.outline)}
	></div>
	{#if fullWidth}
		<div class="flex items-center w-full gap-1.5 px-2 relative z-1">
			<BarsStaggered size={16} />
			<span class="text-2xs font-medium truncate">{summary || 'Group'}</span>
			{#if stepCount != null}
				<span class="text-2xs text-tertiary">{stepCount} node{stepCount !== 1 ? 's' : ''}</span>
			{/if}
			{#if actions}
				<div class="ml-auto flex items-center gap-1">
					{@render actions()}
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col w-full">
			<FlowModuleSchemaItemViewer label={summary || 'Group'} {colorClasses}>
				{#snippet icon()}
					<BarsStaggered size={16} />
				{/snippet}
			</FlowModuleSchemaItemViewer>
		</div>
	{/if}
</div>
