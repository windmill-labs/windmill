<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import FlowBranchAll from './tutorials/FlowBranchAll.svelte'
	import FlowBranchOne from './tutorials/FlowBranchOne.svelte'
	import FlowBuilderTutorialSimpleFlow from './tutorials/FlowBuilderTutorialSimpleFlow.svelte'
	import FlowBuilderTutorialsForLoop from './tutorials/FlowBuilderTutorialsForLoop.svelte'

	let flowBuilderTutorialSimpleFlow: FlowBuilderTutorialSimpleFlow | undefined = undefined
	let flowBuilderTutorialsForLoop: FlowBuilderTutorialsForLoop | undefined = undefined
	let flowBranchOne: FlowBranchOne | undefined = undefined
	let flowBranchAll: FlowBranchAll | undefined = undefined

	export function runTutorialById(id: string) {
		if (id === 'forloop') {
			flowBuilderTutorialsForLoop?.runTutorial()
		} else if (id === 'branchone') {
			flowBranchOne?.runTutorial()
		} else if (id === 'branchall') {
			flowBranchAll?.runTutorial()
		} else if (id === 'action') {
			flowBuilderTutorialSimpleFlow?.runTutorial()
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
/>
<FlowBuilderTutorialsForLoop
	bind:this={flowBuilderTutorialsForLoop}
	on:error
	on:skipAll={skipAll}
/>
<FlowBranchOne bind:this={flowBranchOne} on:error on:skipAll={skipAll} />
<FlowBranchAll bind:this={flowBranchAll} on:error on:skipAll={skipAll} />
