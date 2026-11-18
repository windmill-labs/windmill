<script lang="ts">
	import { page } from '$app/state'
	import AgentDetailPage from '$lib/components/flows/content/AgentDetailPage.svelte'
	import { workspaceStore } from '$lib/stores'
</script>

<!-- Keyed: the route is reused across agents, and the page holds the form's args. -->
{#key `${$workspaceStore}:${page.params.path}`}
	<AgentDetailPage
		path={page.params.path ?? ''}
		workspace={$workspaceStore}
		openConversation={page.url.searchParams.get('conversation') ?? undefined}
	/>
{/key}
