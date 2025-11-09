<script lang="ts">
	import { goto } from '$lib/navigation'

	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'

	async function startSetup(): Promise<void> {
		$workspaceStore = 'admins'
		goto('/user/instance_settings')
	}

	async function decline(): Promise<void> {
		goto('/user/workspaces')
	}
</script>

<CenteredModal title="Welcome to Windmill">
	<p class="text-center text-lg mt-4 mb-4">
		This is a brand new instance. Setup the instance settings, then set the default superadmin user
		and enable hub resource type sync
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button color="light" size="xs2" variant="contained" on:click={decline}>Skip</Button>
		<Button variant="accent" size="lg" on:click={startSetup}>Setup</Button>
	</div>
</CenteredModal>
