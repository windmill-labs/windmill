<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast, sleep } from '$lib/utils'
	import { onMount } from 'svelte'
	import { UserService } from '$lib/gen'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import Icon from 'svelte-awesome'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	let error = $page.url.searchParams.get('error')
	let clientName = $page.params.client_name
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		if (error) {
			sendUserToast(`Error trying to login with ${clientName} ${error}`, true)
			goto('/user/login')
		} else if (code && state && clientName) {
			try {
				await UserService.loginWithOauth({ requestBody: { code, state }, clientName })
			} catch (e) {
				goto('/user/login')
				throw e
			}
			if ($workspaceStore) {
				$userStore = await getUserExt($workspaceStore)
				goto('/')
			} else {
				goto('/user/workspaces')
			}
		} else {
			sendUserToast('Missing code or state as query params', true)
			goto('/user/login')
		}
	})
</script>

<CenteredModal title="Login from {clientName} in progress">
	<div class="mx-auto w-0">
		<Icon class="animate-spin" data={faSpinner} scale={2.0} />
	</div>
</CenteredModal>
