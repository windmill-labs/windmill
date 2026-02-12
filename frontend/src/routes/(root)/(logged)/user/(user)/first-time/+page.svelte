<script lang="ts">
	import { goto } from '$lib/navigation'

	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'

	async function startSetup(advanced = false): Promise<void> {
		$workspaceStore = 'admins'
		goto(advanced ? '/user/instance_settings?mode=full' : '/user/instance_settings')
	}

	async function decline(): Promise<void> {
		goto('/user/workspaces')
	}
</script>

<CenteredModal title="Welcome to Windmill">
	<p class="text-center text-secondary mt-4 mb-4">
		Configure your instance settings to get started. You can use the quick setup for essential
		settings or the advanced setup for full control.
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button color="light" variant="contained" unifiedSize="md" on:click={decline}>Skip</Button>
		<div class="flex items-center gap-2">
			<Button variant="default" unifiedSize="md" on:click={() => startSetup(true)}>Advanced setup</Button>
			<Button variant="accent" unifiedSize="md" on:click={() => startSetup()}>Quick setup</Button>
		</div>
	</div>
</CenteredModal>
