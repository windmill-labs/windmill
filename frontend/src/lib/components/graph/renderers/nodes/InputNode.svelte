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
		flowStore?.val && previewArgs && flowStore?.val?.schema
			? schemaToObject(flowStore?.val.schema as Schema, previewArgs.val || {})
			: undefined
	)
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{#if data.insertable && !data.hasPreprocessor}
			<div class="absolute bottom-full left-0 right-0 flex center-center mb-3.5">
				<InsertModuleButton
					index={0}
					kind="preprocessor"
					on:new={(e) => {
						data?.eventHandlers.insert({
							index: 0,
							kind: e.detail.kind,
							inlineScript: e.detail.inlineScript,
							isPreprocessor: true
						})
					}}
					on:pickScript={(e) => {
						data?.eventHandlers.insert({
							index: 0,
							kind: e.detail.kind,
							script: e.detail,
							isPreprocessor: true
						})
					}}
					clazz="w-[14px] h-[14px]"
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
			onEditInput={data.eventHandlers.editInput}
			onTestFlow={() => {
				data.eventHandlers.testFlow()
			}}
			isRunning={data.isRunning}
			onCancelTestFlow={() => {
				data.eventHandlers.cancelTestFlow()
			}}
			onOpenPreview={() => {
				data.eventHandlers.openPreview()
			}}
			onHideJobStatus={() => {
				data.eventHandlers.hideJobStatus()
			}}
		/>
	{/snippet}
</NodeWrapper>
