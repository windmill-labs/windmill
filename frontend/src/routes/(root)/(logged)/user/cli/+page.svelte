<script lang="ts">
	import { UserService } from '$lib/gen'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'

	let port = Number($page.url.searchParams.get('port'))
	port = port == 0 || Number.isNaN(port) ? 80 : port

	async function authorizeToken(): Promise<void> {
		const newToken = await UserService.createToken({
			requestBody: {
				label: 'External Tool Token',
				expiration: undefined
			}
		})

		const url = 'http://localhost:' + port + '?token=' + newToken + '&workspace=' + $workspaceStore
		window.location.href = url
	}
</script>

<CenteredModal title="Authorize login request">
	<p class="text-center text-lg mb-6">
		Token will be posted to your local machine to port {port}
	</p>
	<div class="flex flex-row justify-around pt-4 gap-x-1">
		<Button variant="border" color="dark" size="sm" href="/">Decline</Button>
		<Button variant="contained" color="blue" size="sm" on:click={authorizeToken}>Authorize</Button>
	</div>
</CenteredModal>
