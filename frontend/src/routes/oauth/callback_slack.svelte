<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Icon from 'svelte-awesome'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'

	let error = $page.url.searchParams.get('error')
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to add slack connection: ${error}`, true)
		} else if (code && state) {
			const res = await OauthService.connectSlackCallback({ requestBody: { code, state } })
			await OauthService.setWorkspaceSlack({ workspace: $workspaceStore!, requestBody: res })
			sendUserToast('Slack workspace added to your Windmill workspace')
		} else {
			sendUserToast('Missing code or state as query params', true)
		}
		goto('/workspace_settings')
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to slack in progress" />
	<div class="mx-auto w-0">
		<Icon class="animate-spin" data={faSpinner} scale={2.0} />
	</div>
</CenteredPage>
