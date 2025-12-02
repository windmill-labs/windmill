<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'

	let troubleshootFlowTutorial: TroubleshootFlowTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		if (id === 'troubleshoot-flow') {
			troubleshootFlowTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<TroubleshootFlowTutorial bind:this={troubleshootFlowTutorial} on:error on:skipAll={skipAll} on:reload />
