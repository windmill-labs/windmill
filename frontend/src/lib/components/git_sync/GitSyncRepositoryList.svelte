<script lang="ts">
	import { getGitSyncContext } from './GitSyncContext.svelte'
	import GitSyncRepositoryCard from './GitSyncRepositoryCard.svelte'
	import { Alert } from '$lib/components/common'

	const gitSyncContext = getGitSyncContext()
</script>

<div class="flex flex-col gap-4">
	{#if gitSyncContext.repositories.length === 0}
		<Alert type="info" title="No repositories configured">
			Add your first Git repository to start syncing your workspace.
		</Alert>
	{:else}
		{#each gitSyncContext.repositories as repository, idx (repository.git_repo_resource_path || idx)}
			<GitSyncRepositoryCard {idx} />
		{/each}
	{/if}
</div>
