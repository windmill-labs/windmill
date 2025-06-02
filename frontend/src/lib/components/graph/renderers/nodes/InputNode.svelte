<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { InputN } from '../../graphBuilder.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import type { FlowEditorContext } from '$lib/components/flows/types'

	interface Props {
		data: InputN['data']
	}

	let { data }: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const { previewArgs, flowStore } =
		getContext<FlowEditorContext | undefined>('FlowEditorContext') || {}

	let topFlowInput = $derived(
		flowStore && previewArgs && $flowStore?.schema
			? schemaToObject($flowStore.schema as Schema, $previewArgs || {})
			: undefined
	)
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{#if data.insertable && !data.hasPreprocessor}
			<div class="absolute bottom-full left-0 right-0 flex center-center mb-3.5">
				<InsertModuleButton
					disableAi={data.disableAi}
					index={data.index ?? 0}
					kind="preprocessor"
					on:new={(e) => {
						data?.eventHandlers.insert({
							index: data.index,
							kind: e.detail.kind,
							inlineScript: e.detail.inlineScript,
							detail: 'preprocessor'
						})
					}}
					on:pickScript={(e) => {
						data?.eventHandlers.insert({
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
			bgHoverColor={getStateHoverColor(undefined, darkMode)}
			on:insert={(e) => {
				setTimeout(() => data?.eventHandlers?.insert(e.detail))
			}}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
			inputJson={topFlowInput}
			prefix="flow_input"
			alwaysPluggable
			cache={data.cache}
			earlyStop={data.earlyStop}
			editMode={data.editMode}
		/>
	{/snippet}
</NodeWrapper>
