<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import AppTutorial from './tutorials/app/AppTutorial.svelte'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = $state(undefined)
	let connectionTutorial: ConnectionTutorial | undefined = $state(undefined)
	let appTutorial: AppTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		if (id === 'backgroundrunnables') {
			backgroundRunnablesTutorial?.runTutorial(options?.skipStepsCount)
		} else if (id === 'connection') {
			connectionTutorial?.runTutorial()
		} else if (id === 'simpleapptutorial') {
			appTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

P
<AppTutorial
	bind:this={appTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={7}
	name="simpleapptutorial"
/>

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
