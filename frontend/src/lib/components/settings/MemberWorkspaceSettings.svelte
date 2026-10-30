<script lang="ts">
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { goto } from '$lib/navigation'
	import { clearStores } from '$lib/storeUtils'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { TriangleAlert } from 'lucide-svelte'

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

	<div
		class="flex flex-col gap-2 rounded-md border border-red-200 bg-red-50 p-4 dark:border-red-700/40 dark:bg-red-900/40"
	>
		<span class="flex items-center gap-2 text-sm font-semibold text-red-800 dark:text-red-100/90">
			<TriangleAlert size={16} class="text-red-500 dark:text-red-300/90" />
			Leave workspace
		</span>
		<p class="text-xs text-red-700 dark:text-red-100/90">
			Leaving removes your access to <b>{workspaceName}</b> right away: its scripts, flows, apps, resources
			and runs. You cannot rejoin by yourself. A workspace admin has to add you again.
		</p>
		<div class="flex">
			<Button
				variant="accent"
				destructive
				unifiedSize="md"
				btnClasses="mt-2"
				onClick={() => (confirmOpen = true)}
			>
				Leave {workspaceName}
			</Button>
		</div>
	</div>
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
