<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import BackgroundRunnablesTutorial from './tutorials/app/BackgroundRunnablesTutorial.svelte'
	import ConnectionTutorial from './tutorials/app/ConnectionTutorial.svelte'
	import ExpressionEvaluationTutorial from './tutorials/app/ExpressionEvaluationTutorial.svelte'

	let backgroundRunnablesTutorial: BackgroundRunnablesTutorial | undefined = undefined
	let connectionTutorial: ConnectionTutorial | undefined = undefined
	let expressionEvaluationTutorial: ExpressionEvaluationTutorial | undefined = undefined

	export function runTutorialById(id: string) {
		if (id === 'backgroundrunnables') {
			backgroundRunnablesTutorial?.runTutorial()
		} else if (id === 'connection') {
			connectionTutorial?.runTutorial()
		} else if (id === 'expressionevaluation') {
			expressionEvaluationTutorial?.runTutorial()
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

<ExpressionEvaluationTutorial
	bind:this={expressionEvaluationTutorial}
	on:error
	on:skipAll={skipAll}
	on:reload
	index={7}
	name="expressionevaluation"
/>
