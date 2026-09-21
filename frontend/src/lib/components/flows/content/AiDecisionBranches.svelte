<script lang="ts">
	import type { AiDecision, FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { Split } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { push } from '$lib/history.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import type { FlowEditorContext } from '../types'
	import {
		branchesForChoiceQuestion,
		choiceBranches,
		decisionChoiceQuestions,
		ensureChoiceArrays
	} from '../branchChoice'
	import ChoiceBranchList from './ChoiceBranchList.svelte'

	interface Props {
		flowModule: FlowModule
		previousModule: FlowModule | undefined
		enableAi?: boolean
	}

	let { flowModule, previousModule, enableAi = false }: Props = $props()

	const { flowStore, history } = getContext<FlowEditorContext>('FlowEditorContext')

	let value = $derived(flowModule.value as AiDecision)
	let questions = $derived(decisionChoiceQuestions(value))
	let picked: string | undefined = $state(undefined)
	let question = $derived(questions.find((q) => q.name === picked) ?? questions[0])
	let toAdd = $derived(question ? branchesForChoiceQuestion(question, choiceBranches(value)) : [])

	function branchOnQuestion() {
		if (toAdd.length === 0) return
		push(history, flowStore.val)
		ensureChoiceArrays(value).branches.push(...toAdd)
		refreshStateStore(flowStore)
	}
</script>

<div class="flex flex-col gap-4">
	<p class="text-xs text-secondary">
		Without branches, this step returns the answers. With branches, the first one whose condition
		holds against the answers runs, else the default, and this step returns that branch's result.
	</p>
	{#if questions.length > 0}
		<div class="flex items-center gap-2">
			<Select
				size="sm"
				class="grow"
				items={questions.map((q) => ({ label: q.name, value: q.name }))}
				bind:value={() => question?.name, (v) => (picked = v)}
			/>
			<Button
				unifiedSize="sm"
				variant="default"
				startIcon={{ icon: Split }}
				disabled={toAdd.length === 0}
				title={toAdd.length === 0
					? 'Every option of this question already has a branch'
					: 'Add one branch per option of this question'}
				on:click={branchOnQuestion}
			>
				Branch on question
			</Button>
		</div>
	{/if}
	<ChoiceBranchList {flowModule} {previousModule} {enableAi} />
</div>
