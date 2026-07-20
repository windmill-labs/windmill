<script lang="ts">
	import { untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import type { FlowModule, InputTransform } from '$lib/gen'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	import type { AgentTool } from '../agentToolUtils'

	let {
		resourceTool,
		toolInputs = $bindable(),
		parentModule = undefined,
		previousModule = undefined,
		enableAi = false,
		forceTestTab = undefined,
		highlightArg = undefined,
		siblingToolNames = undefined
	}: {
		// The tool as defined on the linked agent resource (structure is read-only here). One editor
		// instance is mounted per selected tool (keyed by id upstream), so this stays constant.
		resourceTool: AgentTool
		// The step's host-local tool wiring, keyed by tool id. Bound so edits persist on the step.
		toolInputs: Record<string, Record<string, InputTransform>>
		parentModule?: FlowModule
		previousModule?: FlowModule
		enableAi?: boolean
		forceTestTab?: Record<string, boolean>
		highlightArg?: Record<string, string | undefined>
		siblingToolNames?: string[]
	} = $props()

	// Editable copy: structure from the resource (code hidden via noEditor on the wrapper),
	// input_transforms seeded from the resource then overlaid with the step's existing binding.
	// FlowModuleComponent edits this copy; the effect below mirrors its inputs into tool_inputs.
	let editable = $state<AgentTool>(
		untrack(() => {
			const base = ((resourceTool.value as { input_transforms?: Record<string, InputTransform> })
				.input_transforms ?? {}) as Record<string, InputTransform>
			const overlay = toolInputs?.[resourceTool.id] ?? {}
			return {
				...resourceTool,
				value: { ...resourceTool.value, input_transforms: { ...base, ...overlay } }
			} as AgentTool
		})
	)

	// Mirror the editor's input_transforms into the step's tool_inputs (full map per tool; the
	// runtime overlays it onto the resource tool). Guarded so our own write doesn't re-trigger.
	$effect(() => {
		const its = $state.snapshot(
			(editable.value as { input_transforms?: Record<string, InputTransform> }).input_transforms
		) as Record<string, InputTransform>
		const id = editable.id
		untrack(() => {
			if (!deepEqual(toolInputs?.[id], its)) {
				toolInputs = { ...toolInputs, [id]: its }
			}
		})
	})
</script>

<AgentToolWrapper
	noEditor
	bind:tool={editable}
	{parentModule}
	{previousModule}
	{enableAi}
	{forceTestTab}
	{highlightArg}
	{siblingToolNames}
/>
