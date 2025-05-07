<script lang="ts">
	import { UserService } from '$lib/gen'
	import { page } from '$app/stores'
	import { base } from '$lib/base'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'

	let port = Number($page.url.searchParams.get('port'))
	let host: string = $page.url.searchParams.get('host') || 'localhost'
	let scheme: string = $page.url.searchParams.get('scheme') || 'http'
	port = port == 0 || Number.isNaN(port) ? 80 : port

	async function authorizeToken(): Promise<void> {
		const newToken = await UserService.createToken({
			requestBody: {
				label: 'External tool token',
				expiration: undefined
			}
		})

		const url =
			scheme + '://' + host + ':' + port + '?token=' + newToken + '&workspace=' + $workspaceStore
		window.location.href = url
	}
</script>

<CenteredModal title="Authorize login request">
	<p class="text-center text-lg mb-6">
		Token will be posted to {host == 'localhost' ? 'your local machine' : host} to port {port}
	</p>
	<div class="flex flex-row justify-around pt-4 gap-x-1">
		<Button variant="border" color="dark" size="sm" href={base}>Decline</Button>
		<Button variant="contained" color="blue" size="sm" on:click={authorizeToken}>Authorize</Button>
	</div>
</CenteredModal>
