<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = undefined

	export function runTutorialById(id: string) {
		if (id === 'backgroundrunnables') {
			backgroundRunnablesTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<BackgroundRunnablesTutorial
	bind:this={backgroundRunnablesTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={5}
	name="backgroundrunnables"
/>
