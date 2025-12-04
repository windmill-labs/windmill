<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = $state(undefined)
	let connectionTutorial: ConnectionTutorial | undefined = $state(undefined)

	// Map tutorial IDs to their component instances
	const tutorialInstances = new Map<
		string,
		{ runTutorial: (options?: number) => void } | undefined
	>()

	// Update map when instances change
	$effect(() => {
		tutorialInstances.set('backgroundrunnables', backgroundRunnablesTutorial)
		tutorialInstances.set('connection', connectionTutorial)
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

<BackgroundRunnablesTutorial
	bind:this={backgroundRunnablesTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={getTutorialIndex('backgroundrunnables')}
	name="backgroundrunnables"
/>

<ConnectionTutorial
	bind:this={connectionTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={getTutorialIndex('connection')}
	name="connection"
/>
