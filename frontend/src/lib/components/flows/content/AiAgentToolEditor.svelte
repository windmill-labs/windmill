<script lang="ts">
	import type { AiAgentTool } from '../agentToolUtils'
	import { validateToolName } from '$lib/components/graph/renderers/nodes/AIToolNode.svelte'
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { getStepPropPicker } from '../previousResults'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import Label from '$lib/components/Label.svelte'

	interface Props {
		tool: AiAgentTool
		parentModule?: FlowModule | undefined
		previousModule?: FlowModule | undefined
		enableAi?: boolean
	}

	let {
		tool = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		enableAi = false
	}: Props = $props()

	const { flowStateStore, flowStore, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let stepPropPicker = $derived(
		getStepPropPicker(
			flowStateStore.val,
			parentModule,
			previousModule,
			tool.id,
			flowStore.val,
			previewArgs.val,
			false
		)
	)

	let nameValid = $derived(validateToolName(tool.summary ?? ''))
</script>

<div class="flex flex-col gap-4 p-4">
	<div class="w-full">
		<Label label="Tool Name">
			<input
				type="text"
				bind:value={tool.summary}
				placeholder="Tool name (alphanumeric and underscores)"
				class="text-sm w-full"
				class:border-red-500={!nameValid && (tool.summary?.length ?? 0) > 0}
			/>
			{#if !nameValid && (tool.summary?.length ?? 0) > 0}
				<p class="text-xs text-red-500 mt-1">
					Tool name must only contain alphanumeric characters and underscores
				</p>
			{/if}
		</Label>
	</div>

	<PropPickerWrapper
		pickableProperties={stepPropPicker.pickableProperties}
		extraLib={stepPropPicker.extraLib}
	>
		<InputTransformSchemaForm
			class="px-2 xl:px-4 pb-8"
			pickableProperties={stepPropPicker.pickableProperties}
			schema={flowStateStore.val[tool.id]?.schema ?? {}}
			previousModuleId={previousModule?.id}
			bind:args={tool.value.input_transforms}
			extraLib={stepPropPicker.extraLib}
			{enableAi}
			isAgentTool={true}
		/>
	</PropPickerWrapper>
</div>
