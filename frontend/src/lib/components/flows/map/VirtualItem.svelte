<script lang="ts">
	import { Badge } from '$lib/components/common'

	import { getContext } from 'svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import VirtualItemWrapper from './VirtualItemWrapper.svelte'

	export let label: string | undefined = undefined
	export let bgColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let id: string | undefined = undefined
	export let center = true
	export let borderColor: string | undefined = undefined
	export let hideId: boolean = false
	export let preLabel: string | undefined = undefined

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

<VirtualItemWrapper
	{label}
	{bgColor}
	{selected}
	{selectable}
	{id}
	onTop={label === 'Input' && $copilotCurrentStepStore === 'Input'}
	on:select
>
	<div
		style={borderColor ? `border-color: ${borderColor};` : 'border: 0'}
		class="flex gap-1 justify-between {center
			? 'items-center'
			: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400 dark:border-gray-600"
	>
		{#if $$slots.icon}
			<slot name="icon" />
			<span class="mr-2" />
		{/if}
		<div />
		<div class="flex flex-col w-full">
			{#if label}
				<div class="truncate text-center">{label}</div>
			{/if}
			{#if preLabel}
				<div class="truncate text-2xs text-center"><pre>{preLabel}</pre></div>
			{/if}
		</div>
		<div class="flex items-center space-x-2">
			{#if id && !hideId}
				<Badge color="indigo">{id}</Badge>
			{/if}
		</div>
	</div>
</VirtualItemWrapper>
