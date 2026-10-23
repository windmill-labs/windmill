<script lang="ts">
	import { page } from '$app/state'
	import AgentDetailPage from '$lib/components/flows/content/AgentDetailPage.svelte'
	import { workspaceStore } from '$lib/stores'
</script>

<AgentDetailPage path={page.params.path ?? ''} workspace={$workspaceStore} />
