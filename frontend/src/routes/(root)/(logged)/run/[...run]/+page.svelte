<script lang="ts">
	import { page } from '$app/state'
	import { goto } from '$lib/navigation'
	import { workspaceStore } from '$lib/stores'
	import RunDetail from '$lib/components/runs/RunDetail.svelte'
</script>

<RunDetail
	id={page.params.run ?? ''}
	workspace={$workspaceStore}
	viewToken={page.url.searchParams.get('view_token') ?? undefined}
	onNavigate={goto}
/>
