<script lang="ts">
	import { goto } from '$app/navigation'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'

	import { sendUserToast } from '$lib/utils'

	sendUserToast('Sucess. You can return to your terminal now.')
	goto('/')
</script>

<CenteredModal title="Redirecting to Home">
	<div class="w-full">
		<div class="block m-auto w-20">
			<WindmillIcon height="80px" width="80px" spin="fast" />
		</div>
	</div>
</CenteredModal>
