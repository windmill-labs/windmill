<script lang="ts">
	import { Badge } from '$lib/components/common'

	import { getContext } from 'svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import VirtualItemWrapper from './VirtualItemWrapper.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import FlowPropPicker from '$lib/components/flows/propPicker/FlowPropPicker.svelte'

	export let label: string | undefined = undefined
	export let bgColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let id: string | undefined = undefined
	export let center = true
	export let borderColor: string | undefined = undefined
	export let hideId: boolean = false
	export let preLabel: string | undefined = undefined
	export let inputJson = {}
	export let prefix = ''

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = propPickerContext?.flowPropPickerConfig
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
		class="flex flex-row gap-1 justify-between {center
			? 'items-center'
			: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400 dark:border-gray-600"
	>
		{#if $$slots.icon}
			<slot name="icon" />
			<span class="mr-2" />
		{/if}
		<div class="flex flex-col flex-grow shrink-0 max-w-full min-w-0">
			{#if label}
				<div class="truncate text-center">{label}</div>
			{/if}
			{#if preLabel}
				<div class="truncate text-2xs text-center"><pre>{preLabel}</pre></div>
			{/if}
		</div>
		{#if id && !hideId}
			<div class="flex items-center shrink min-w-0">
				<Badge color="indigo" wrapperClass="w-full" baseClass="max-w-full" title={id}>
					<span class="max-w-full text-2xs truncate">{id}</span>
				</Badge>
			</div>
		{/if}
	</div>
	{#if $flowPropPickerConfig && Object.keys(inputJson).length > 0}
		<div class="absolute -bottom-[12px] right-[50%] translate-x-[50%] center-center">
			<FlowPropPicker json={inputJson} {prefix} />
		</div>
	{/if}
</VirtualItemWrapper>
