<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'

	let error = $page.url.searchParams.get('error')
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to add slack connection: ${error}`, true)
		} else if (code && state) {
			await OauthService.connectSlackCallbackInstance({
				requestBody: { code, state }
			})
			sendUserToast('Slack workspace connected to your Windmill instance.')
		} else {
			sendUserToast('Missing code or state as query params', true)
		}
		goto('/#superadmin-settings')
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to slack in progress" />
	<div class="mx-auto w-0">
		<WindmillIcon height="80px" width="80px" spin="fast" />
	</div>
</CenteredPage>
