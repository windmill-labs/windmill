<script lang="ts">
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Loader2 } from 'lucide-svelte'

	let client_name = $page.params.client_name
	let error = $page.url.searchParams.get('error')
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			window.opener.postMessage(
				{ type: 'error', error: `Error trying to add ${client_name} connection: ${error}` },
				'*'
			)
			// goto('/resources')
		} else if (code && state) {
			try {
				const res = await OauthService.connectCallback({
					clientName: client_name,
					requestBody: { code, state }
				})
				window.opener.postMessage({ type: 'success', res, resource_type: client_name }, '*')
				// goto(`/resources?resource_type=${client_name}`)
			} catch (e) {
				window.opener.postMessage(
					{ type: 'error', error: `Error parsing the response token, ${e.body}` },
					'*'
				)
				// goto('/resources')
			}
		} else {
			window.opener.postMessage(
				{ type: 'error', error: 'Missing code or state as query params' },
				'*'
			)
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
