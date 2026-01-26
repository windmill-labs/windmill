<script lang="ts">
	import { goto } from '$lib/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'

	import { sendUserToast } from '$lib/toast'

	sendUserToast('Success. You can return to your terminal now.')
	goto('/')
</script>

<CenteredModal title="Redirecting to Home" loading={true}></CenteredModal>
