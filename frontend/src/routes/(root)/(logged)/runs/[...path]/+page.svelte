<!-- TODO : Refactor the runs page to separate state from UI so I don't need to do the {#key trick} -->

<script lang="ts">
	import { page } from '$app/state'
	import RunsPage from '../../../../../lib/components/RunsPage.svelte'

	// Get the path from route params (e.g., /runs/u/user/script → "u/user/script")
	let initialPath = $derived(page.params.path ?? '')
</script>

<RunsPage {initialPath} />
