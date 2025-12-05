<script lang="ts">
	import TutorialRouter from './tutorials/TutorialRouter.svelte'
	import FlowBuilderLiveTutorial from './tutorials/FlowBuilderLiveTutorial.svelte'
	import TroubleshootFlowTutorial from './tutorials/TroubleshootFlowTutorial.svelte'

	let tutorialRouter: TutorialRouter | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		tutorialRouter?.runTutorialById(id)
	}
</script>

<TutorialRouter
	bind:this={tutorialRouter}
	tutorials={[
		{
			id: 'flow-live-tutorial',
			component: FlowBuilderLiveTutorial
		},
		{
			id: 'troubleshoot-flow',
			component: TroubleshootFlowTutorial
		}
	]}
/>