<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Alert } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	let success = $page.url.searchParams.get('success') === 'true'

	let attempt = 0
	if (!success) {
		setTimeout(() => {
			goto('/workspace_settings?tab=premium')
		}, 5000)
	} else {
		let interval = setInterval(async () => {
			attempt += 1
			if ((await WorkspaceService.getSettings({ workspace: $workspaceStore! })).customer_id) {
				clearInterval(interval)
				goto('/workspace_settings?tab=premium')
			} else if (attempt > 10) {
				sendUserToast('Subscription upgrade failed. Contact contact@windmill.dev', true)
				clearInterval(interval)
				goto('/workspace_settings?tab=premium')
			}
		}, 5000)
	}
</script>

<!-- svelte-ignore missing-declaration -->
<CenteredModal title="Subscription upgrade {success ? 'succeeded' : 'failed'}">
	{#if !success}
		<div class="my-2">
			<Alert type="error" title="Checkout failed">
				The checkout failed, your subscription has not been updated.
			</Alert>
		</div>
		<p class="text-sm my-6 text-primary">
			You will be redirected to the workspace settings page in 5 seconds...
		</p>
	{:else}
		<p class="text-sm my-6 text-primary w-full text-center">
			Waiting for your upgrade to be processed...
		</p>
	{/if}

	<div class="block m-auto w-20">
		<WindmillIcon height="80px" width="80px" spin="fast" />
	</div>
</CenteredModal>
