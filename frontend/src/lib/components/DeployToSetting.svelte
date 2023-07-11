<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	export let workspaceToDeployTo: string | undefined

	$: deployableWorkspaces = $usersWorkspaceStore?.workspaces
		.map((w) => w.id)
		.filter((w) => w != $workspaceStore)
</script>

<h3 class="mt-8">Workspace to link to</h3>
<div class="flex min-w-0 mt-2">
	<select
		bind:value={workspaceToDeployTo}
		on:change={async (e) => {
			await WorkspaceService.editDeployTo({
				workspace: $workspaceStore ?? '',
				requestBody: { deploy_to: workspaceToDeployTo == '' ? undefined : workspaceToDeployTo }
			})
			if (workspaceToDeployTo == '') {
				workspaceToDeployTo = undefined
				sendUserToast('Disabled setting deployable workspace')
			} else {
				sendUserToast('Set deployable workspace to ' + workspaceToDeployTo)
			}
		}}
	>
		{#if deployableWorkspaces?.length == 0}
			<option disabled>No workspace deployable to</option>
		{/if}
		<option value="">Disable deployment</option>
		{#each deployableWorkspaces ?? [] as name}
			<option value={name}>{name}</option>
		{/each}
	</select>
</div>
