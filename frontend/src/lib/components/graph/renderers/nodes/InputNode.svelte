<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'

	export let data: {
		hasPreprocessor: boolean
		insertable: boolean
		modules: FlowModule[]
		moving: string | undefined
		eventHandlers: GraphEventHandlers
		index: number
		disableAi: boolean
		disableMoveIds: string[]
		cache: boolean
		earlyStop: boolean
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const pickablePropertiesFiltered = propPickerContext?.pickablePropertiesFiltered

	function filterIterFromInput(inputJson: Record<string, any> | undefined): Record<string, any> {
		if (!inputJson || typeof inputJson !== 'object') return {}

		const newJson = { ...inputJson }
		delete newJson.iter

		return newJson
	}

	$: filteredInput = filterIterFromInput($pickablePropertiesFiltered?.flow_input)
</script>

<NodeWrapper let:darkMode>
	{#if data.insertable && !data.hasPreprocessor}
		<div class="absolute bottom-full left-0 right-0 flex center-center mb-3.5">
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
				class="w-[14px] h-[14px]"
				iconSize={10}
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
		inputJson={filteredInput}
		prefix="flow_input"
		alwaysPluggable
		cache={data.cache}
		earlyStop={data.earlyStop}
	/>
</NodeWrapper>
