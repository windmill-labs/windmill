<!-- TODO : Refactor the runs page to separate state from UI so I don't need to do the {#key trick} -->

<script lang="ts">
	import { page } from '$app/state'
	import { onMount } from 'svelte'
	import RunsPage from '../../../../../lib/components/RunsPage.svelte'
	import RunsTutorial from '$lib/components/tutorials/RunsTutorial.svelte'

	let runsTutorial: RunsTutorial

	onMount(() => {
		// Check if there's a tutorial parameter in the URL
		const tutorialParam = page.url.searchParams.get('tutorial')
		if (tutorialParam === 'runs-tutorial') {
			// Small delay to ensure page is fully loaded
			setTimeout(() => {
				runsTutorial?.runTutorial()
			}, 500)
		}
	})
</script>

<RunsPage />

<RunsTutorial bind:this={runsTutorial} index={7} />
