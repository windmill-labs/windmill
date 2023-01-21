<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Alert } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'

	let success = $page.url.searchParams.get('success') === 'true'

	let attempt = 0
	if (!success) {
		setTimeout(() => {
			goto('/workspace_settings?tab=premium')
		}, 5000)
	} else {
		setInterval(async () => {
			attempt += 1
			if ((await WorkspaceService.getSettings({ workspace: $workspaceStore! })).customer_id) {
				goto('/workspace_settings?tab=premium')
			} else if (attempt > 10) {
				sendUserToast('Subscription upgrade failed. Contact contact@windmill.dev', true)
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
	{/if}
	<p class="text-sm my-6 text-gray-600">
		You will be redirected to the workspace settings page in 5 seconds...
	</p>
	<div class="block m-auto w-20">
		<WindmillIcon class="animate-[spin_6s_linear_infinite]" height="80px" width="80px" />
	</div>
</CenteredModal>
