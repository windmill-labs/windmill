<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import FlowCopilotButton from '$lib/components/flows/map/FlowCopilotButton.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'

	export let data: {
		hasPreprocessor: boolean
		insertable: boolean
		modules: FlowModule[]
		moving: string | undefined
		eventHandlers: GraphEventHandlers
		index: number
		enableTrigger: boolean
		disableAi: boolean
		disableMoveIds: string[]
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
			<InsertModuleButton
				disableAi={data.disableAi}
				index={data.index ?? 0}
				modules={data?.modules ?? []}
				kind="preprocessor"
				on:new={(e) => {
					data?.eventHandlers.insert({
						modules: data.modules,
						index: data.index,
						kind: e.detail.kind,
						inlineScript: e.detail.inlineScript,
						detail: 'preprocessor'
					})
				}}
				on:pickScript={(e) => {
					data?.eventHandlers.insert({
						modules: data.modules,
						index: data.index,
						script: e.detail,
						detail: 'preprocessor'
					})
				}}
			/>
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
