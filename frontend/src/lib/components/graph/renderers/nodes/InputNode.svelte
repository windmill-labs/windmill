<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { InputN } from '../../graphBuilder.svelte'
	import { getContext } from 'svelte'

	import InsertModulePopover from '$lib/components/flows/map/InsertModulePopover.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import DiffActionBar from '$lib/components/flows/map/DiffActionBar.svelte'
	import { schemaToObject } from '$lib/schema'
	import type { Schema } from '$lib/common'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { MessageSquare } from 'lucide-svelte'
	import { getGraphContext } from '../../graphContext'
	import FunnelCog from '$lib/components/icons/FunnelCog.svelte'

	interface Props {
		data: InputN['data']
	}

	let { data }: Props = $props()

	const { selectionManager, diffManager } = getGraphContext()

	const flowEditorContext = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	const { previewArgs, flowStore } = flowEditorContext || {}

	let topFlowInput = $derived(
		flowStore?.val && previewArgs && flowStore?.val?.schema
			? schemaToObject(flowStore?.val.schema as Schema, previewArgs.val || {})
			: undefined
	)

	let inputLabel = $derived(data.chatInputEnabled ? 'Chat message' : 'Input')
</script>

<DiffActionBar
	moduleId="Input"
	moduleAction={data.moduleAction}
	{diffManager}
	flowStore={flowEditorContext?.flowStore}
/>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{#if data.insertable && !data.hasPreprocessor}
			<div class="absolute bottom-full left-0 right-0 center-center mb-2.5">
				<InsertModulePopover
					disableAi={data.disableAi}
					allowTrigger
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
				>
					{#snippet trigger()}
						<InsertModuleButton
							title={`Add preprocessor step`}
							id={`flow-editor-add-step-0`}
							Icon={FunnelCog}
						/>
					{/snippet}
				</InsertModulePopover>
			</div>
		{/if}
		<VirtualItem
			id={'Input'}
			hideId={true}
			label={inputLabel}
			selectable
			selected={selectionManager?.isNodeSelected('Input')}
			on:insert={(e) => {
				setTimeout(() => data?.eventHandlers?.insert(e.detail))
			}}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
			inputJson={topFlowInput}
			prefix="flow_input"
			nodeKind="input"
			cache={data.cache}
			earlyStop={data.earlyStop}
			editMode={data.editMode}
			action={data.moduleAction?.action}
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
			individualStepTests={data.individualStepTests}
			job={data.flowJob}
			showJobStatus={data.showJobStatus}
			flowHasChanged={data.flowHasChanged}
		>
			{#snippet icon()}
				{#if data.chatInputEnabled}
					<MessageSquare size={14} class="text-blue-500 dark:text-blue-400" />
				{/if}
			{/snippet}
		</VirtualItem>
	{/snippet}
</NodeWrapper>
