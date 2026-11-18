<script lang="ts">
	import { page } from '$app/state'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import ScriptDetail from '$lib/components/details/ScriptDetail.svelte'

	// Read once, as the fragment is only ever set by the link that opened this page
	// (`Run again`, which prefills the run form from it) and never updated in place.
	const locationHash = window.location.hash
</script>

<ScriptDetail
	hash={page.params.hash ?? ''}
	workspace={$workspaceStore}
	searchParams={page.url.searchParams}
	{locationHash}
	onNavigate={goto}
	active={true}
/>
