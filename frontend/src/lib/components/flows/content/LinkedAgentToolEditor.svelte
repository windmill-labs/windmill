<script lang="ts">
	import { untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import type { FlowModule, InputTransform } from '$lib/gen'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	import type { AgentTool } from '../agentToolUtils'
	import { toolInputOverrides } from '../agentResourceUtils'
	import HighlightCode from '$lib/components/HighlightCode.svelte'

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

	// The tool's code lives in the resource and is read-only here; surface it for reference.
	let code = $derived(
		resourceTool.value as { type?: string; language?: string; content?: string } | undefined
	)

	// The resource tool's own transforms (authored against the source flow). Overrides are stored as
	// the diff from these, so unchanged inputs keep inheriting from the resource.
	const baseInputs =
		(untrack(() =>
			$state.snapshot(
				(resourceTool.value as { input_transforms?: Record<string, InputTransform> })
					.input_transforms
			)
		) as Record<string, InputTransform>) ?? {}

	// Editable copy: structure from the resource (code hidden via noEditor on the wrapper),
	// input_transforms seeded from the resource then overlaid with the step's existing binding.
	// FlowModuleComponent edits this copy; the effect below mirrors its diff into tool_inputs.
	let editable = $state<AgentTool>(
		untrack(() => {
			const overlay = toolInputs?.[resourceTool.id] ?? {}
			return {
				...resourceTool,
				value: { ...resourceTool.value, input_transforms: { ...baseInputs, ...overlay } }
			} as AgentTool
		})
	)

	// Mirror the editor's inputs into the step's tool_inputs, storing only the keys that diverge from
	// the resource tool (see toolInputOverrides). This makes merely opening a tool a no-op — the seed
	// equals base ∪ overrides, whose diff is the saved overrides — and lets a revert persist. Guarded
	// so our own write doesn't re-trigger.
	$effect(() => {
		const its = $state.snapshot(
			(editable.value as { input_transforms?: Record<string, InputTransform> }).input_transforms
		) as Record<string, InputTransform>
		const id = editable.id
		const overrides = toolInputOverrides(its, baseInputs)
		untrack(() => {
			if (!deepEqual(toolInputs?.[id], overrides)) {
				toolInputs = { ...toolInputs, [id]: overrides }
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

{#if code?.type === 'rawscript' && code?.content}
	<details class="mx-2 mt-2 rounded-md border border-border bg-surface-secondary xl:mx-4">
		<summary class="cursor-pointer px-3 py-2 text-2xs font-medium text-tertiary">
			Tool code (read-only)
		</summary>
		<div class="max-h-64 overflow-auto border-t border-border p-2 text-xs">
			<HighlightCode language={code.language} code={code.content} />
		</div>
	</details>
{/if}
