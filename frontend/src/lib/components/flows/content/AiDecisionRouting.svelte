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
		decisionChoiceQuestions,
		findRouting,
		missingOptions
	} from '../aiDecisionBranching'
	import { getModuleArrayContainer } from '../flowTree'
	import { insertNewModuleAtIndex } from '../flowStateUtils.svelte'

	interface Props {
		flowModule: FlowModule
		class?: string
	}

	let { flowModule, class: className = '' }: Props = $props()

	const { flowStore, flowStateStore, history, selectionManager } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let questions = $derived(
		decisionChoiceQuestions(flowModule)
			.filter((q) => q.options.length > 0)
			.map((q) => {
				const routing = findRouting(flowStore.val, flowModule.id, q.name)
				return {
					...q,
					routing,
					missing: routing ? missingOptions(routing, flowModule.id, q.name, q.options) : q.options
				}
			})
			.filter((q) => q.missing.length > 0)
	)

	async function branchOn(question: string, options: string[]) {
		const container = getModuleArrayContainer(flowStore.val.value, flowModule.id)
		if (!container) return
		push(history, flowStore.val)
		const modules = (await insertNewModuleAtIndex(
			flowStore,
			flowStateStore,
			container.modules,
			container.index + 1,
			'branchone'
		)) as FlowModule[]
		const routing = modules[container.index + 1]
		addChoiceBranches(routing, flowModule.id, question, options)
		refreshStateStore(flowStore)
		selectionManager.selectId(routing.id)
	}

	function addMissing(routing: FlowModule, question: string, missing: string[]) {
		push(history, flowStore.val)
		addChoiceBranches(routing, flowModule.id, question, missing)
		refreshStateStore(flowStore)
	}
</script>

{#if questions.length > 0}
	<div class="flex flex-col gap-2 {className}">
		{#each questions as q (q.name)}
			{#if !q.routing}
				<Button
					unifiedSize="sm"
					variant="default"
					wrapperClasses="self-start"
					startIcon={{ icon: Split }}
					title="Add a Branch to one after this step, with one branch per option"
					onClick={() => branchOn(q.name, q.options)}
				>
					Branch on {q.name}
				</Button>
			{:else}
				<Alert
					type="info"
					size="xs"
					title="The {q.name} question has {q.missing.length === 1
						? 'an option'
						: 'options'} with no branch in {q.routing.id}: {q.missing.join(', ')}"
					actions={[
						{
							label: q.missing.length === 1 ? 'Add branch' : 'Add branches',
							onClick: () => addMissing(q.routing!, q.name, q.missing)
						}
					]}
				/>
			{/if}
		{/each}
	</div>
{/if}
