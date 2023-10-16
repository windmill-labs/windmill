<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = undefined
	let connectionTutorial: ConnectionTutorial | undefined = undefined

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		if (id === 'backgroundrunnables') {
			backgroundRunnablesTutorial?.runTutorial(options?.skipStepsCount)
		} else if (id === 'connection') {
			connectionTutorial?.runTutorial()
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

<ConnectionTutorial
	bind:this={connectionTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={6}
	name="connection"
/>
