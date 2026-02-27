<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	interface Props {
		onreload?: (...args: any[]) => any
		onerror?: (...args: any[]) => any
	}

	let {
		onreload = undefined,
		onerror = undefined
	}: Props = $props()

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

<TroubleshootFlowTutorial
	bind:this={troubleshootFlowTutorial}
	index={getTutorialIndex('troubleshoot-flow')}
	onerror={onerror}
	onskipAll={skipAll}
	onreload={onreload}
/>
