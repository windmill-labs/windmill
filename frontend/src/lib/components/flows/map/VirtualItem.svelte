<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'

	export let label: string | undefined = undefined
	export let bgColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let id: string | undefined = undefined
	export let center = true
	export let borderColor: string | undefined = undefined
	export let hideId: boolean = false
	export let preLabel: string | undefined = undefined

	const dispatch = createEventDispatcher<{
		insert: {
			script?: { path: string; summary: string; hash: string | undefined }
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger' | 'move'
			modules: FlowModule[]
			index: number
		}
		select: string
	}>()

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={classNames(
		'w-full flex relative overflow-hidden rounded-sm',
		selectable ? 'cursor-pointer' : '',
		selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : '',
		label === 'Input' && $copilotCurrentStepStore === 'Input' ? 'z-[901]' : ''
	)}
	style="width: 275px; max-height: 34px; background-color: {bgColor} !important;"
	on:click={() => {
		if (selectable) {
			if (id) {
				dispatch('select', id)
			} else {
				dispatch('select', label || label || '')
			}
		}
	}}
	title={(label ? label + ' ' : '') + (label ?? '')}
	id={`flow-editor-virtual-${encodeURIComponent(label || label || '')}`}
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
</div>
