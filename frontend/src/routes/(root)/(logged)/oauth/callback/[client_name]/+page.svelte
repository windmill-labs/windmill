<script lang="ts">
	import { page } from '$app/stores'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	let client_name = $page.params.client_name
	let error = $page.url.searchParams.get('error')
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to add ${client_name} connection: ${error}`, true)
			const message = {
				type: 'error',
				error: `Error trying to add ${client_name} connection: ${error}`
			}
			if (window.opener) {
				window.opener?.postMessage(message, '*')
			} else {
				localStorage.setItem('oauth-callback', JSON.stringify(message))
			}

			// goto('/resources')
		} else if (code && state) {
			try {
				const extraParams = Object.fromEntries(
					Array.from($page.url.searchParams.entries()).filter(
						([key]) => key !== 'code' && key !== 'state' && key !== 'error'
					)
				)
				const res = await OauthService.connectCallback({
					clientName: client_name,
					requestBody: { code, state }
				})
				const message = { type: 'success', res, resource_type: client_name, extra: extraParams }
				sendUserToast('successful', false)
				if (window.opener) {
					console.log('Sending oauth popup message')
					window.opener?.postMessage(message, '*')
				} else {
					console.log('Storing oauth popup message in local storage')
					localStorage.setItem('oauth-callback', JSON.stringify(message))
				}
				// goto(`/resources?resource_type=${client_name}`)
			} catch (e) {
				sendUserToast(`Error trying to add ${client_name} connection: ${e.body}`, true)
				const message = { type: 'error', error: `Error parsing the response token, ${e.body}` }
				if (window.opener) {
					console.log('Sending oauth popup message')
					window.opener?.postMessage(message, '*')
				} else {
					console.log('Storing oauth popup message in local storage')
					localStorage.setItem('oauth-callback', JSON.stringify(message))
				}

				// goto('/resources')
			}
		} else {
			sendUserToast('Missing code or state as query params', true)
			const message = { type: 'error', error: 'Missing code or state as query params' }
			if (window.opener) {
				console.log('Sending oauth popup message')
				window.opener?.postMessage(message, '*')
			} else {
				console.log('Storing oauth popup message in local storage')
				localStorage.setItem(
					'oauth-callback',
					JSON.stringify({ type: 'error', error: 'Missing code or state as query params' })
				)
			}
			// goto('/resources')
		}
		close()
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to {client_name} in progress" />
	<div class="mx-auto w-0">
		<Loader2 class="animate-spin" />
	</div>
</CenteredPage>
