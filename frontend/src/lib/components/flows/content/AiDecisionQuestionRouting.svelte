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
		quoteOptions,
		decisionChoiceQuestions,
		findRouting,
		missingOptions
	} from '../aiDecisionBranching'
	import { findModuleInFlow } from '../flowTree'
	import { branchOnQuestion } from '../aiDecisionInsert'
	import StepIdBadge from './StepIdBadge.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { graphBranchIndex, removeBranch } from '../branchOps'
	import { dfs } from '../dfs'

	interface Props {
		decisionId: string
		question: string
	}

	let { decisionId, question }: Props = $props()

	const ctx = getContext<FlowEditorContext>('FlowEditorContext')
	const { flowStore, flowStateStore, history, selectionManager } = ctx

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

	// Steps inside the stale branches, which removing them deletes along with them.
	let staleSteps = $derived.by(() => {
		if (routing?.value.type !== 'branchone') return 0
		const branches = routing.value.branches
		return stale.reduce((n, s) => n + dfs(branches[s.index]?.modules ?? [], (m) => m.id).length, 0)
	})
	let confirmingRemoval = $state(false)

	function removeStale(target: FlowModule) {
		// From the last, so each index still points at its branch once the ones after it are gone.
		for (const { index } of [...stale].sort((a, b) => b.index - a.index)) {
			removeBranch(target.id, graphBranchIndex('branchone', index), {
				flowStore,
				flowStateStore,
				history
			})
		}
		refreshStateStore(flowStore)
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
			onClick={() => branchOnQuestion(ctx, decisionId, question, options)}
		>
			Branch on this question
		</Button>
	{/if}
{:else}
	{@const target = routing}
	<div class="flex items-center gap-2 text-xs text-secondary">
		<Split size={12} />
		<span>Branched in</span>
		<StepIdBadge id={target.id} />
		<Button
			variant="default"
			unifiedSize="xs"
			title="Open the Branch to one"
			onClick={() => selectionManager.selectId(target.id)}
		>
			Open
		</Button>
	</div>
	{#if missing.length > 0}
		<Alert
			type="info"
			size="xs"
			title="{missing.length === 1 ? 'An option has' : 'Options have'} no branch yet: {quoteOptions(
				missing
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
			title="{stale.length === 1
				? 'A branch handles an option'
				: 'Branches handle options'} this question no longer has: {quoteOptions(
				stale.map((s) => s.option)
			)}"
			actions={[
				{
					label: stale.length === 1 ? 'Remove branch' : 'Remove branches',
					onClick: () => (staleSteps > 0 ? (confirmingRemoval = true) : removeStale(target))
				}
			]}
		/>
		{@const one = stale.length === 1}
		{@const steps = staleSteps === 1 ? '1 step' : `${staleSteps} steps`}
		<ConfirmationModal
			open={confirmingRemoval}
			title="Remove the {one ? 'branch' : 'branches'} and {steps}?"
			confirmationText="Remove"
			onCanceled={() => (confirmingRemoval = false)}
			onConfirmed={() => {
				confirmingRemoval = false
				removeStale(target)
			}}
		>
			The {one ? 'branch' : 'branches'} for {quoteOptions(stale.map((s) => s.option))} in
			<StepIdBadge id={target.id} />
			{one ? 'contains' : 'contain'}
			{steps}, which will be deleted too.
		</ConfirmationModal>
	{/if}
{/if}
