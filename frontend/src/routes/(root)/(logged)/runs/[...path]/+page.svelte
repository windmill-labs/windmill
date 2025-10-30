<!-- TODO : Refactor the runs page to separate state from UI so I don't need to do the {#key trick} -->

<script>
	import { page } from '$app/state'
	import RunsPage, { DEFAULT_RUNS_PER_PAGE } from '../../../../../lib/components/RunsPage.svelte'
	let perPage = $state(
		parseInt(page.url.searchParams.get('per_page') ?? DEFAULT_RUNS_PER_PAGE.toString())
	)
</script>

{#key perPage}
	<RunsPage bind:perPage />
{/key}
