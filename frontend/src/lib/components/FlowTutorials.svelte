<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderTutorialBranchAll from './tutorials/FlowBuilderTutorialBranchAll.svelte'
	import FlowBuilderTutorialBranchOne from './tutorials/FlowBuilderTutorialBranchOne.svelte'
	import FlowBuilderTutorialSimpleFlow from './tutorials/FlowBuilderTutorialSimpleFlow.svelte'
	import FlowBuilderTutorialForLoop from './tutorials/FlowBuilderTutorialForLoop.svelte'
	import FlowBuilderTutorialErrorHandler from './tutorials/FlowBuilderTutorialErrorHandler.svelte'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'

	let flowBuilderTutorialSimpleFlow: FlowBuilderTutorialSimpleFlow | undefined = $state(undefined)
	let flowBuilderTutorialForLoop: FlowBuilderTutorialForLoop | undefined = $state(undefined)
	let flowBuilderTutorialBranchOne: FlowBuilderTutorialBranchOne | undefined = $state(undefined)
	let flowBuilderTutorialBranchAll: FlowBuilderTutorialBranchAll | undefined = $state(undefined)
	let flowBuilderTutorialErrorHandler: FlowBuilderTutorialErrorHandler | undefined =
		$state(undefined)
	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined =
		$state(undefined)
	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string, indexToInsertAt?: number | undefined) {
		if (id === 'forloop') {
			flowBuilderTutorialForLoop?.runTutorial(indexToInsertAt)
		} else if (id === 'branchone') {
			flowBuilderTutorialBranchOne?.runTutorial()
		} else if (id === 'branchall') {
			flowBuilderTutorialBranchAll?.runTutorial()
		} else if (id === 'action') {
			flowBuilderTutorialSimpleFlow?.runTutorial()
		} else if (id === 'error-handler') {
			flowBuilderTutorialErrorHandler?.runTutorial()
		} else if (id === 'flow-live-tutorial') {
			flowBuilderLiveTutorial?.runTutorial()
		} else if (id === 'troubleshoot-flow') {
			troubleshootFlowTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<FlowBuilderTutorialSimpleFlow
	bind:this={flowBuilderTutorialSimpleFlow}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<FlowBuilderTutorialForLoop
	bind:this={flowBuilderTutorialForLoop}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<FlowBuilderTutorialBranchOne
	bind:this={flowBuilderTutorialBranchOne}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<FlowBuilderTutorialBranchAll
	bind:this={flowBuilderTutorialBranchAll}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<FlowBuilderTutorialErrorHandler
	bind:this={flowBuilderTutorialErrorHandler}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<FlowBuilderLiveTutorial
	bind:this={flowBuilderLiveTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<TroubleshootFlowTutorial bind:this={troubleshootFlowTutorial} on:error on:skipAll={skipAll} on:reload />
