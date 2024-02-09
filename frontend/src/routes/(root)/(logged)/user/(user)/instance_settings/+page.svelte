<script lang="ts">
	import { goto } from '$app/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import InstanceSettings from '$lib/components/InstanceSettings.svelte'
	import { Button } from '$lib/components/common'
</script>

<CenteredModal title="Instance Settings">
	<InstanceSettings />
	<p class="text-secondary text-sm px-2 py-4">
		You can change these settings later in the instance settings but finishing setup will leave this
		page.
	</p>
	<Button
		on:click={() => {
			goto('/apps/get/g/all/setup_app?nomenubar=true')
		}}>Finish Setup</Button
	>
</CenteredModal>
