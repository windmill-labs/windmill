<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import AppTutorial from './tutorials/app/AppTutorial.svelte'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = $state(undefined)
	let connectionTutorial: ConnectionTutorial | undefined = $state(undefined)
	let appTutorial: AppTutorial | undefined = $state(undefined)

	// Map tutorial IDs to their component instances
	const tutorialInstances = new Map<
		string,
		{ runTutorial: (options?: number) => void } | undefined
	>()

	// Update map when instances change
	$effect(() => {
		tutorialInstances.set('backgroundrunnables', backgroundRunnablesTutorial)
		tutorialInstances.set('connection', connectionTutorial)
		tutorialInstances.set('simpleapptutorial', appTutorial)
	})

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		const instance = tutorialInstances.get(id)
		if (instance) {
			if (options?.skipStepsCount !== undefined) {
				instance.runTutorial(options.skipStepsCount)
			} else {
				instance.runTutorial()
			}
		} else {
			console.warn(`Tutorial instance not found for id: ${id}`)
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

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
