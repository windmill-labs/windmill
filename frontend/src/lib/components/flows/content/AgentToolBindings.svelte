<script lang="ts">
	import { untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import type { InputTransform } from '$lib/gen'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Wrench } from 'lucide-svelte'
	import type { PickableProperties } from '../previousResults'
	import {
		agentToolToFlowModule,
		isFlowModuleTool,
		type AgentTool,
		type FlowModuleTool
	} from '../agentToolUtils'
	import { toolInputOverrides } from '../agentResourceUtils'
	import { loadSchemaFromModule } from '../flowInfers'

	let {
		tools,
		toolInputs = $bindable(),
		pickableProperties = undefined,
		extraLib = 'missing extraLib',
		workspace = undefined
	}: {
		// Tools inherited from the linked agent resource (read-only structure).
		tools: AgentTool[]
		// Host-local wiring authored on the step, keyed by tool id then input key. Only the diff from
		// the resource tool's own transforms is stored (see toolInputOverrides).
		toolInputs: Record<string, Record<string, InputTransform>>
		pickableProperties?: PickableProperties
		extraLib?: string
		workspace?: string
	} = $props()

	let flowTools = $derived(tools.filter(isFlowModuleTool))

	// Tool input schemas, inferred from each tool's definition (rawscript content / script path).
	let schemas = $state<Record<string, any>>({})
	$effect(() => {
		const ts = flowTools
		const ws = workspace
		untrack(() => {
			for (const t of ts) {
				if (schemas[t.id] === undefined) {
					loadSchemaFromModule(agentToolToFlowModule(t), ws)
						.then(({ schema }) => {
							schemas[t.id] = schema
						})
						.catch(() => {
							schemas[t.id] = { properties: {} }
						})
				}
			}
		})
	})

	function baseInputs(tool: FlowModuleTool): Record<string, InputTransform> {
		return ((tool.value as { input_transforms?: Record<string, InputTransform> })
			.input_transforms ?? {}) as Record<string, InputTransform>
	}

	// Editable copies (resource base ∪ saved overrides), one per tool. The form mutates these
	// proxies in place, so a plain function-bind getter returning a fresh merged object would lose
	// edits — the mirror effect below diffs them back into the step's tool_inputs instead.
	let localArgs = $state<Record<string, Record<string, InputTransform>>>({})
	$effect(() => {
		const ts = flowTools
		untrack(() => {
			for (const t of ts) {
				if (localArgs[t.id] === undefined) {
					localArgs[t.id] = { ...baseInputs(t), ...(toolInputs?.[t.id] ?? {}) }
				}
			}
		})
	})

	// Mirror edits into the step, storing only the diff from the resource tool. Seeding is a no-op
	// (the seed's diff equals the saved overrides), so merely opening the step doesn't dirty the flow.
	$effect(() => {
		const snap = $state.snapshot(localArgs) as Record<string, Record<string, InputTransform>>
		untrack(() => {
			for (const t of flowTools) {
				const cur = snap[t.id]
				if (!cur) continue
				const overrides = toolInputOverrides(cur, baseInputs(t))
				if (!deepEqual(toolInputs?.[t.id] ?? {}, overrides)) {
					toolInputs = { ...toolInputs, [t.id]: overrides }
				}
			}
		})
	})

	function toolCode(tool: FlowModuleTool): { language?: string; content?: string } | undefined {
		const v = tool.value as { type?: string; language?: string; content?: string }
		return v.type === 'rawscript' && v.content ? v : undefined
	}
</script>

{#if flowTools.length > 0}
	<div class="flex flex-col gap-2 px-2 pb-8 xl:px-4">
		{#each flowTools as tool (tool.id)}
			{@const code = toolCode(tool)}
			<div class="rounded-md border border-border">
				<div class="flex items-center gap-1.5 border-b border-border px-3 py-1.5 text-xs">
					<Wrench size={14} class="shrink-0 text-tertiary" />
					<span class="font-medium">{tool.summary || tool.id}</span>
					{#if tool.description}
						<span class="truncate text-2xs text-tertiary" title={tool.description}
							>{tool.description}</span
						>
					{/if}
				</div>
				{#if schemas[tool.id] !== undefined && localArgs[tool.id] !== undefined}
					<InputTransformSchemaForm
						class="px-3 pt-1"
						schema={schemas[tool.id]}
						{pickableProperties}
						{extraLib}
						isAgentTool
						bind:args={
							() => localArgs[tool.id],
							(v) => {
								localArgs[tool.id] = v
							}
						}
					/>
				{:else}
					<div class="px-3 py-2 text-2xs text-tertiary">Loading inputs...</div>
				{/if}
				{#if code}
					<details class="border-t border-border">
						<summary class="cursor-pointer px-3 py-1.5 text-2xs font-medium text-tertiary">
							Tool code (read-only)
						</summary>
						<div class="max-h-64 overflow-auto border-t border-border p-2 text-xs">
							<HighlightCode language={code.language} code={code.content ?? ''} />
						</div>
					</details>
				{/if}
			</div>
		{/each}
	</div>
{/if}
