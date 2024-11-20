<script lang="ts">
	import { goto } from '$lib/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import InstanceSettings from '$lib/components/InstanceSettings.svelte'
	import { Button } from '$lib/components/common'

	let saved = false
</script>

<CenteredModal large title="Instance settings">
	<InstanceSettings on:saved={() => (saved = true)} />
	<p class="text-secondary text-sm px-2 py-4">
		You can change these settings later in the instance settings but finishing setup will leave this
		page.
	</p>
	<Button
		disabled={!saved}
		on:click={() => {
			goto('/apps/get/g/all/setup_app?nomenubar=true&workspace=admins')
		}}>Finish Setup {!saved ? '(save settings at least once)' : ''}</Button
	>
</CenteredModal>
