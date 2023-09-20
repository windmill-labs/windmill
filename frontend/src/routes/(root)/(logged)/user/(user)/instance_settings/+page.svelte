<script lang="ts">
	import { goto } from '$app/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import InstanceSettings from '$lib/components/InstanceSettings.svelte'
</script>

<CenteredModal title="Instance Settings">
	<InstanceSettings on:save={() => goto('/apps/get/g/all/setup_app')} />
</CenteredModal>
