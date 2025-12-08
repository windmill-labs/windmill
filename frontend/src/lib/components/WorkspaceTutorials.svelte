<script lang="ts">
	import TutorialRouter from './tutorials/TutorialRouter.svelte'
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'

	let tutorialRouter: TutorialRouter | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		tutorialRouter?.runTutorialById(id)
	}
</script>

<TutorialRouter
	bind:this={tutorialRouter}
	tutorials={[
		{
			id: 'workspace-onboarding',
			component: WorkspaceOnboardingTutorial
		}
	]}
/>
