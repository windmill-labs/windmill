<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
</script>

<CenteredModal>
	There was an error during login

	<Button on:click={() => window.close()}>close</Button>
</CenteredModal>
