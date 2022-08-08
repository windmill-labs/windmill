<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'
	import { onMount } from 'svelte'
	import { UserService } from '$lib/gen'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import Icon from 'svelte-awesome'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { logoutWithRedirect } from '$lib/logout'

	let error = $page.url.searchParams.get('error')
	let clientName = $page.params.client_name
	let code = $page.url.searchParams.get('code') ?? undefined
	let state = $page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		const rd = localStorage.getItem('rd')
		if (rd) {
			localStorage.removeItem('rd')
		}
		if (error) {
			sendUserToast(`Error trying to login with ${clientName} ${error}`, true)
			logoutWithRedirect(rd ?? undefined)
		} else if (code && state && clientName) {
			try {
				await UserService.loginWithOauth({ requestBody: { code, state }, clientName })
			} catch (e) {
				logoutWithRedirect(rd ?? undefined)
				sendUserToast(e.body ?? e.message, true)
				return
			}
			if ($workspaceStore) {
				$userStore = await getUserExt($workspaceStore)
				goto(rd ?? '/')
			} else {
				if (rd) {
					goto('/user/workspaces?rd=' + encodeURIComponent(rd))
				} else {
					goto('/user/workspaces')
				}
			}
		} else {
			sendUserToast('Missing code or state as query params', true)
			logoutWithRedirect(rd ?? undefined)
		}
	})
</script>

<CenteredModal title="Login from {clientName} in progress">
	<div class="mx-auto w-0">
		<Icon class="animate-spin" data={faSpinner} scale={2.0} />
	</div>
</CenteredModal>
