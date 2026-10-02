<script lang="ts">
	import { getContext } from 'svelte'
	import { Split } from 'lucide-svelte'
	import type { FlowModule } from '$lib/gen'
	import { Alert, Button } from '$lib/components/common'
	import { push } from '$lib/history.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import type { FlowEditorContext } from '../types'
	import {
		addChoiceBranches,
		checkRouting,
		decisionChoiceQuestions,
		findRouting,
		missingOptions
	} from '../aiDecisionBranching'
	import { findModuleInFlow, getModuleArrayContainer } from '../flowTree'
	import { insertNewModuleAtIndex } from '../flowStateUtils.svelte'

	interface Props {
		decisionId: string
		question: string
	}

	let { decisionId, question }: Props = $props()

	const { flowStore, flowStateStore, history, selectionManager } =
		getContext<FlowEditorContext>('FlowEditorContext')

	// Read from the stored step, so what this offers matches what the branches are checked against.
	let options = $derived.by(() => {
		const decision = findModuleInFlow(flowStore.val.value, decisionId)
		return (
			(decision && decisionChoiceQuestions(decision).find((q) => q.name === question)?.options) ??
			[]
		)
	})
	let routing = $derived(findRouting(flowStore.val, decisionId, question))
	let missing = $derived(routing ? missingOptions(routing, decisionId, question, options) : options)
	let stale = $derived(
		routing
			? (checkRouting(flowStore.val, routing).find(
					(c) => c.decisionId === decisionId && c.question === question
				)?.stale ?? [])
			: []
	)

	async function branchOn() {
		const container = getModuleArrayContainer(flowStore.val.value, decisionId)
		if (!container) return
		push(history, flowStore.val)
		const modules = (await insertNewModuleAtIndex(
			flowStore,
			flowStateStore,
			container.modules,
			container.index + 1,
			'branchone'
		)) as FlowModule[]
		const inserted = modules[container.index + 1]
		addChoiceBranches(inserted, decisionId, question, options)
		refreshStateStore(flowStore)
		selectionManager.selectId(inserted.id)
	}

	function addMissing(target: FlowModule) {
		push(history, flowStore.val)
		addChoiceBranches(target, decisionId, question, missing)
		refreshStateStore(flowStore)
	}
</script>

{#if !routing}
	{#if options.length > 0}
		<Button
			unifiedSize="sm"
			variant="default"
			wrapperClasses="self-start"
			startIcon={{ icon: Split }}
			title="Add a Branch to one after this step, with one branch per option"
			onClick={branchOn}
		>
			Branch on this question
		</Button>
	{/if}
{:else}
	{@const target = routing}
	<div class="flex items-center gap-1 text-xs text-secondary">
		<Split size={12} />
		<span>Branched in</span>
		<Button
			variant="subtle"
			unifiedSize="xs"
			title="Open the Branch to one"
			onClick={() => selectionManager.selectId(target.id)}
		>
			{target.summary || target.id}
		</Button>
	</div>
	{#if missing.length > 0}
		<Alert
			type="info"
			size="xs"
			title="{missing.length === 1 ? 'An option has' : 'Options have'} no branch yet: {missing.join(
				', '
			)}"
			actions={[
				{
					label: missing.length === 1 ? 'Add branch' : 'Add branches',
					onClick: () => addMissing(target)
				}
			]}
		/>
	{/if}
	{#if stale.length > 0}
		<Alert
			type="warning"
			size="xs"
			title="{target.summary || target.id} has {stale.length === 1
				? 'a branch'
				: 'branches'} for {stale
				.map((s) => s.option)
				.join(', ')}, which this question no longer offers"
			actions={[{ label: 'Open', onClick: () => selectionManager.selectId(target.id) }]}
		/>
	{/if}
{/if}
