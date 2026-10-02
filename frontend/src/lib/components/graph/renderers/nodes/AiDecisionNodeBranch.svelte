<script lang="ts">
	import { getContext } from 'svelte'
	import { Split } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { decisionChoiceQuestions, findRouting } from '$lib/components/flows/aiDecisionBranching'
	import { branchOnQuestion } from '$lib/components/flows/aiDecisionInsert'
	import { findModuleInFlow } from '$lib/components/flows/flowTree'

	interface Props {
		moduleId: string
	}

	let { moduleId }: Props = $props()

	// Absent in graphs drawn outside the flow editor, which have nothing to insert into.
	const ctx = getContext<FlowEditorContext | undefined>('FlowEditorContext')

	// The first question nothing branches on yet; the step's question cards offer the rest.
	let question = $derived.by(() => {
		if (!ctx) return undefined
		const decision = findModuleInFlow(ctx.flowStore.val.value, moduleId)
		if (!decision) return undefined
		return decisionChoiceQuestions(decision).find(
			(q) => q.options.length > 0 && !findRouting(ctx.flowStore.val, moduleId, q.name)
		)
	})
</script>

{#if ctx && question}
	{@const q = question}
	<div class="absolute top-1/2 -right-10 -translate-y-1/2 z-10">
		<Button
			variant="default"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: Split }}
			title="Branch on {q.name}"
			onClick={() => branchOnQuestion(ctx, moduleId, q.name, q.kind, q.options)}
		/>
	</div>
{/if}
