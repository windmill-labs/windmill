<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let flowBuilderLiveTutorial: FlowBuilderLiveTutorial | undefined = $state(undefined)
	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	// Map tutorial IDs to their component instances
	const tutorialInstances = new Map<string, { runTutorial: () => void } | undefined>()

	// Update map when instances change
	$effect(() => {
		tutorialInstances.set('flow-live-tutorial', flowBuilderLiveTutorial)
		tutorialInstances.set('troubleshoot-flow', troubleshootFlowTutorial)
	})

	export function runTutorialById(id: string) {
		const instance = tutorialInstances.get(id)
		if (instance) {
			instance.runTutorial()
		} else {
			console.warn(`Tutorial instance not found for id: ${id}`)
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