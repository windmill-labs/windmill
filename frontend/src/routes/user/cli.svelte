<script lang="ts">
	import { goto } from '$app/navigation'

	import { UserService } from '$lib/gen'
	import { page } from '$app/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'

	let port = Number($page.url.searchParams.get('port'))
	port = port == 0 || port == NaN ? 80 : port

	async function authorizeToken(): Promise<void> {
		const newToken = await UserService.createToken({
			requestBody: {
				label: 'External Tool Token',
				expiration: undefined
			}
		})

		await goto('http://localhost:' + port + '?token=' + newToken)
	}
</script>

<!-- Enable submit form on enter -->

<CenteredModal title="Authorize this request for a Token">
	<p class="text-center text-lg">
		Token will be posted to your local machine to port {port}
	</p>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="border" size="sm" href="/user/workspaces">Decline</Button>
		<button
			class="place-items-end bg-blue-500 hover:bg-blue-700 text-white font-bold py-1 px-2 border rounded"
			type="button"
			on:click={authorizeToken}
		>
			Authorize
		</button>
	</div>
</CenteredModal>
