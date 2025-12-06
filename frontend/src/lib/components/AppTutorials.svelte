<script lang="ts">
	import TutorialRouter from './tutorials/TutorialRouter.svelte'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'

	let tutorialRouter: TutorialRouter | undefined = $state(undefined)

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		tutorialRouter?.runTutorialById(id, options)
	}
</script>

<TutorialRouter
	bind:this={tutorialRouter}
	tutorials={[
		{
			id: 'backgroundrunnables',
			component: BackgroundRunnablesTutorial,
			name: 'backgroundrunnables',
			supportsSkipSteps: true
		},
		{
			id: 'connection',
			component: ConnectionTutorial,
			name: 'connection',
			supportsSkipSteps: true
		}
	]}
/>
