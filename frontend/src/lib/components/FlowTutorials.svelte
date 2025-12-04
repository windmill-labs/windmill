<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined =
		$state(undefined)
	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		if (id === 'flow-live-tutorial') {
			flowBuilderLiveTutorial?.runTutorial()
		} else if (id === 'troubleshoot-flow') {
			troubleshootFlowTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}

	// Get indexes from config
	const flowLiveTutorialIndex = getTutorialIndex('flow-live-tutorial')
	const troubleshootFlowTutorialIndex = getTutorialIndex('troubleshoot-flow')
</script>

<FlowBuilderLiveTutorial
	bind:this={flowBuilderLiveTutorial}
	index={flowLiveTutorialIndex}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<TroubleshootFlowTutorial 
	bind:this={troubleshootFlowTutorial}
	index={troubleshootFlowTutorialIndex}
	on:error 
	on:skipAll={skipAll} 
	on:reload 
/>