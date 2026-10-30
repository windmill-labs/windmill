<script lang="ts">
	import { Alert } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { goto } from '$lib/navigation'
	import { clearStores } from '$lib/storeUtils'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	const workspaceName = $derived(
		$userWorkspaces.find((w) => w.id === $workspaceStore)?.name ?? $workspaceStore ?? ''
	)

	let confirmOpen = $state(false)

	async function leave() {
		await WorkspaceService.leaveWorkspace({ workspace: $workspaceStore ?? '' })
		sendUserToast(`You left ${workspaceName}`)
		clearStores()
		await goto('/user/workspaces')
	}
</script>

<div class="flex flex-col gap-6 max-w-2xl py-6">
	<div class="flex flex-col gap-1">
		<span class="text-sm font-semibold text-emphasis">You are a member of {workspaceName}</span>
		<p class="text-xs text-secondary">
			Only workspace admins can change its settings. Ask one of them if something here needs to
			change.
		</p>
	</div>

	<Alert
		type="error"
		title="Leave workspace"
		actions={[
			{ label: `Leave ${workspaceName}`, variant: 'accent', onClick: () => (confirmOpen = true) }
		]}
	>
		Leaving removes your access to <b>{workspaceName}</b> right away: its scripts, flows, apps, resources
		and runs. You cannot rejoin by yourself. A workspace admin has to add you again.
	</Alert>
</div>

<ConfirmationModal
	open={confirmOpen}
	title="Leave {workspaceName}?"
	confirmationText="Leave workspace"
	onCanceled={() => (confirmOpen = false)}
	onConfirmed={() => {
		confirmOpen = false
		void leave()
	}}
>
	<p>
		You will lose access to <b>{workspaceName}</b>. Only a workspace admin can add you back.
	</p>
</ConfirmationModal>
