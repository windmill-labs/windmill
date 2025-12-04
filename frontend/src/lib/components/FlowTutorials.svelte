<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined = $state(undefined)
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

</script>

<FlowBuilderLiveTutorial
	bind:this={flowBuilderLiveTutorial}
	index={getTutorialIndex('flow-live-tutorial')}
	on:error
	on:skipAll={skipAll}
	on:reload
/>
<TroubleshootFlowTutorial 
	bind:this={troubleshootFlowTutorial}
	index={getTutorialIndex('troubleshoot-flow')}
	on:error 
	on:skipAll={skipAll} 
	on:reload 
/>