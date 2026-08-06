<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import { oauthStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Loader2 } from 'lucide-svelte'

	const client_name = 'supabase_wizard'

	let error = page.url.searchParams.get('error')
	let code = page.url.searchParams.get('code') ?? undefined
	let state = page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to fetch projects from windmill: ${error}`, true)
			goto('/resources')
		} else if (code && state) {
			try {
				const res = await OauthService.connectCallback({
					clientName: client_name,
					requestBody: { code, state }
				})
				$oauthStore = res
				// The data table wizard parks its state before redirecting; send the user back to
				// where they started rather than always to /resources.
				const returnTo = sessionStorage.getItem('datatable_wizard_resume')
					? '/workspace_settings?tab=windmill_data_tables'
					: '/resources'
				goto(`${returnTo}${returnTo.includes('?') ? '&' : '?'}callback=${client_name}`)
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
	<PageHeader title="Connection to supabase in progress" />
	<div class="mx-auto w-0">
		<Loader2 class="animate-spin" />
	</div>
</CenteredPage>
