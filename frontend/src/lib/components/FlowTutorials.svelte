<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'

	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined =
		$state(undefined)
	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string, indexToInsertAt?: number | undefined) {
		if (id === 'flow-live-tutorial') {
			flowBuilderLiveTutorial?.runTutorial()
		} else if (id === 'troubleshoot-flow') {
			troubleshootFlowTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<FlowBuilderLiveTutorial
	bind:this={flowBuilderLiveTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<TroubleshootFlowTutorial 
	bind:this={troubleshootFlowTutorial} 
	on:error 
	on:skipAll={skipAll} 
	on:reload 
/>