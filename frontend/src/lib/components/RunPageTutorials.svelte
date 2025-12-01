<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import ExploreRunsTutorial from './tutorials/ExploreRunsTutorial.svelte'

	let exploreRunsTutorial: ExploreRunsTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		if (id === 'explore-runs') {
			exploreRunsTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<ExploreRunsTutorial bind:this={exploreRunsTutorial} on:error on:skipAll={skipAll} on:reload />
