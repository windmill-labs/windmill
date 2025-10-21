<script lang="ts">
	import { page } from '$app/stores'
	import WorkspaceMigration from '$lib/components/settings/WorkspaceMigration.svelte'
	import { workspaceStore } from '$lib/stores'

	const sourceWorkspace = $derived($page.url.searchParams.get('source') || '')

	$effect(() => {
		// Redirect if no source workspace provided
		if (!sourceWorkspace) {
			window.location.href = `/workspace_settings?workspace=${$workspaceStore}`
		}
	})
</script>

<div class="max-w-4xl mx-auto p-6">
	{#if sourceWorkspace}
		<WorkspaceMigration {sourceWorkspace} />
	{:else}
		<div class="flex items-center justify-center p-8">
			<p class="text-secondary">Loading...</p>
		</div>
	{/if}
</div>
