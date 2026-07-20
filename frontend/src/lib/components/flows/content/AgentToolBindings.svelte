<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import type { InputTransform } from '$lib/gen'
	import { hostBoundToolInputs, type AgentTool } from '../agentResourceUtils'

	let {
		tools,
		toolInputs = $bindable()
	}: {
		// Tools inherited from the linked agent resource.
		tools: AgentTool[]
		// Host-local wiring authored on the step, keyed by tool id then input key.
		toolInputs: Record<string, Record<string, InputTransform>>
	} = $props()

	let bindings = $derived(hostBoundToolInputs(tools))

	function currentExpr(toolId: string, key: string, fallback: string): string {
		const t = toolInputs?.[toolId]?.[key]
		return t?.type === 'javascript' ? (t.expr ?? '') : fallback
	}

	function setExpr(toolId: string, key: string, expr: string) {
		toolInputs = {
			...toolInputs,
			[toolId]: { ...toolInputs?.[toolId], [key]: { type: 'javascript', expr } as InputTransform }
		}
	}
</script>

{#if bindings.length > 0}
	<div class="mt-1 rounded-md border border-border bg-surface-secondary px-3 py-2">
		<div class="text-2xs font-medium text-tertiary uppercase tracking-wide">Tool inputs</div>
		<p class="text-2xs text-tertiary mt-0.5">
			Wire the agent's tool inputs to this flow. Reference <span class="font-mono">flow_input</span
			>, <span class="font-mono">results.&lt;step&gt;</span> or
			<span class="font-mono">previous_result</span>.
		</p>
		<div class="mt-2 flex flex-col gap-3">
			{#each bindings as b (b.toolId)}
				<div class="flex flex-col gap-1">
					<div class="text-2xs font-medium text-secondary truncate" title={b.toolId}>{b.label}</div>
					{#each b.keys as k (k.key)}
						<div class="flex flex-col gap-0.5">
							<span class="text-2xs text-tertiary font-mono">{k.key}</span>
							<div class="rounded border border-border overflow-hidden">
								<SimpleEditor
									lang="javascript"
									code={currentExpr(b.toolId, k.key, k.resourceExpr)}
									on:change={(e) => setExpr(b.toolId, k.key, e.detail.code)}
									autoHeight
									small
									fixedOverflowWidgets
									class="text-2xs"
								/>
							</div>
						</div>
					{/each}
				</div>
			{/each}
		</div>
	</div>
{/if}
