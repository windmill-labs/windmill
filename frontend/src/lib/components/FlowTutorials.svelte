<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'
	import { TUTORIALS_CONFIG } from '$lib/tutorials/config'

	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined =
		$state(undefined)
	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	// Helper function to get tutorial index from config
	function getTutorialIndex(id: string): number {
		for (const tab of Object.values(TUTORIALS_CONFIG)) {
			const tutorial = tab.tutorials.find((t) => t.id === id)
			if (tutorial?.index !== undefined) return tutorial.index
		}
		throw new Error(`Tutorial index not found for id: ${id}. Make sure the tutorial has an index defined in config.`)
	}

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