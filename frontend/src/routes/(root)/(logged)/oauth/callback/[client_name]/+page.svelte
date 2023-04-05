<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import { oauthStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Loader2 } from 'lucide-svelte'

	let client_name = $page.params.client_name
	let error = $page.url.searchParams.get('error')
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to add ${client_name} connection: ${error}`, true)
			goto('/resources')
		} else if (code && state) {
			try {
				const res = await OauthService.connectCallback({
					clientName: client_name,
					requestBody: { code, state }
				})
				$oauthStore = res
				goto(`/resources?resource_type=${client_name}`)
			} catch (e) {
				sendUserToast(`Error parsing the response token, ${e.body}`, true)
				goto('/resources')
			}
		} else {
			sendUserToast('Missing code or state as query params', true)
			goto('/resources')
		}
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to {client_name} in progress" />
	<div class="mx-auto w-0">
		<Loader2 class="animate-spin" />
	</div>
</CenteredPage>
