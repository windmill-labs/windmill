<script lang="ts">
	import { page } from '$app/state'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import FlowDetail from '$lib/components/details/FlowDetail.svelte'

	// Read once, as the fragment is only ever set by the link that opened this page
	// (`Run again`, which prefills the run form from it) and never updated in place.
	const locationHash = window.location.hash
</script>

<FlowDetail
	path={page.params.path ?? ''}
	workspace={$workspaceStore}
	searchParams={page.url.searchParams}
	{locationHash}
	onNavigate={goto}
	active={true}
/>
