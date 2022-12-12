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

<CenteredModal title="Welcome to Windmill!">
	<p class="text-center text-lg">
		Looks like this is a brand new instance. We've prepared a script for you to get a proper account
		and get you setup with the Hub. Should I take you there now?
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="border" size="sm" href="/" on:click={decline}>Decline</Button>
		<button
			class="place-items-end bg-blue-500 hover:bg-blue-700 text-white font-bold py-1 px-2 border rounded"
			type="button"
			on:click={startSetup}
		>
			Accept
		</button>
	</div>
</CenteredModal>
