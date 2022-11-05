<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import { workspaceStore, oauthStore } from '$lib/stores'
	import Icon from 'svelte-awesome'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
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
			await OauthService.connectSlackCallback({
				workspace: $workspaceStore!,
				requestBody: { code, state }
			})
			sendUserToast('Slack workspace connected to your Windmill workspace')
		} else {
			sendUserToast('Missing code or state as query params', true)
		}
		goto('/workspace_settings')
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to slack in progress" />
	<div class="mx-auto w-0">
		<WindmillIcon class="animate-[spin_5s_linear_infinite]" height="80px" width="80px" />
	</div>
</CenteredPage>
