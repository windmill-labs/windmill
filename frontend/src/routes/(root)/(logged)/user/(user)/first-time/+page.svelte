<script lang="ts">
	import { goto } from '$app/navigation'

	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'

	async function startSetup(): Promise<void> {
		$workspaceStore = 'admins'
		goto('/apps/get/g/all/setup_app')
	}

	async function decline(): Promise<void> {
		goto('/user/workspaces')
	}
</script>

<CenteredModal title="Welcome to Windmill">
	<p class="text-center text-lg mt-4">
		This is a brand new instance. In the superadmins workspace, there is an app to setup an account
		and configure syncing the resource types of this instance with the hub periodically. Go there?
		You can always go back to this app by going to the superadmins workspace.
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="border" size="sm" on:click={decline}>Skip setup</Button>
		<button
			class="place-items-end bg-blue-500 hover:bg-blue-700 text-white font-bold py-1 px-2 border rounded"
			type="button"
			on:click={startSetup}
		>
			Setup
		</button>
	</div>
</CenteredModal>
