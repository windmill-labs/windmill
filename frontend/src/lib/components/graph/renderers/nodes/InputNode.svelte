<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import { Cross } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import FlowCopilotButton from '$lib/components/flows/map/FlowCopilotButton.svelte'

	export let data: {
		insertable: boolean
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		disableAi: boolean
		hasPreprocessor: boolean
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper let:darkMode>
	{#if data.insertable && !data.disableAi && !data.hasPreprocessor}
		<FlowCopilotButton className="-top-[4.25rem]" />
	{/if}
	{#if data.insertable && !data.hasPreprocessor}
		<div class="absolute -top-8 left-1/2 transform -translate-x-1/2 z-10">
			<button
				on:click={(e) => {
					data.eventHandlers?.insert({
						modules: data.modules,
						detail: 'preprocessor'
					})
				}}
				title="Add preprocessor step"
				id={`flow-editor-add-preprocessor`}
				type="button"
				class={twMerge(
					'w-5 h-5 flex items-center justify-center',
					'outline-[1px] outline dark:outline-gray-500 outline-gray-300',
					'text-secondary',
					'bg-surface focus:outline-none hover:bg-surface-hover   rounded '
				)}
			>
				<Cross size={12} />
			</button>
		</div>
	{/if}
	<VirtualItem
		label="Input"
		selectable
		selected={$selectedId === 'Input'}
		bgColor={getStateColor(undefined, darkMode)}
		on:insert={(e) => {
			data.eventHandlers?.insert(e.detail)
		}}
		on:select={(e) => {
			data.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
