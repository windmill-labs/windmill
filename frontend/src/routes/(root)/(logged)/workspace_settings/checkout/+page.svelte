<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Alert } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'

	let success = $page.url.searchParams.get('success') === 'true'

	if (!success) {
		setTimeout(() => {
			goto('/workspace_settings?tab=plan')
		}, 5000)
	}
</script>

<!-- svelte-ignore missing-declaration -->
<CenteredModal title="Checkout">
	<div class="w-full ">
		<div class="block m-auto w-20">
			<Alert role="error" title="Checkout failed">
				The checkout failed, your subscription has not been updated.
			</Alert>
			<p class="text-sm"> You will be redirected to the workspace settings page in 5 seconds... </p>
			<WindmillIcon class="animate-[spin_6s_linear_infinite]" height="80px" width="80px" />
		</div>
	</div>
</CenteredModal>
