<script lang="ts">
	import { Badge } from '$lib/components/common'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { takeFirstVisit, timeOfDayGreeting } from './greeting'

	// `name` is the full name when the account has one; the greeting uses the first word.
	const userName = $derived($userStore?.name?.trim().split(/\s+/)[0] || $userStore?.username)
	const workspaceName = $derived(
		$userWorkspaces.find((w) => w.id === $workspaceStore)?.name ?? $workspaceStore
	)
	// Read once the membership for this workspace has loaded: `userStore` still describes the
	// previous workspace for a moment after a switch.
	const firstVisit = $derived(
		!!$workspaceStore &&
			$userStore?.workspace_id === $workspaceStore &&
			takeFirstVisit($workspaceStore, $userStore.created_at)
	)
	const greeting = timeOfDayGreeting()
</script>

<div class="flex items-center justify-center gap-2 mb-4">
	<p class="text-center font-regular text-3xl">
		{#if firstVisit}
			Welcome to {workspaceName}{userName ? `, ${userName}` : ''}
		{:else}
			{greeting}{userName ? `, ${userName}` : ''}
		{/if}
	</p>
	<Badge color="blue" small>Beta</Badge>
</div>
