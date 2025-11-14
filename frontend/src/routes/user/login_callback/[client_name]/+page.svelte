<script lang="ts">
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { UserService, WorkspaceService } from '$lib/gen'
	import CenteredModal from '$lib/components/CenteredModal.svelte'

	import { userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { logoutWithRedirect } from '$lib/logout'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { parseQueryParams } from '$lib/utils'
	import { page } from '$app/state'
	import { isCloudHosted } from '$lib/cloud'
	import { getCookie } from '$lib/cookies'
	// import { getAndDeleteCookie } from '$lib/cookies'

	let error = page.url.searchParams.get('error')
	let clientName = page.params.client_name ?? ''
	let code = page.url.searchParams.get('code') ?? undefined
	let state = page.url.searchParams.get('state') ?? undefined

	onMount(async () => {
		// const closeCookie = getAndDeleteCookie('close')
		// console.log('closeCookie', closeCookie)
		const rd = localStorage.getItem('rd')
		if (rd) {
			localStorage.removeItem('rd')
		}
		const cookieCloseUponLogin = getCookie('close') == 'true'
		const closeUponLogin = cookieCloseUponLogin ?? localStorage.getItem('closeUponLogin') == 'true'
		if (error) {
			sendUserToast(`Error trying to login with ${clientName} ${error}`, true)
			if (closeUponLogin) {
				goto('/user/close')
				return
			}
			await logoutWithRedirect(rd ?? undefined)
		} else if (code && state && clientName) {
			try {
				await UserService.loginWithOauth({ requestBody: { code, state }, clientName })
			} catch (e) {
				if (closeUponLogin) {
					goto('/user/close')
					return
				}
				await logoutWithRedirect(rd ?? undefined)
				sendUserToast(e.body ?? e.message, true)
				return
			}

			if (rd?.startsWith('http')) {
				if (closeUponLogin) {
					closeUponLoginSuccess()
					return
				}
				window.location.href = rd
				return
			}

			// Check if this is a first-time user (individual user onboarding)
			// Only show onboarding for cloud-hosted instances
			if (isCloudHosted()) {
				try {
					const globalUserInfo = await UserService.globalWhoami()
					if (globalUserInfo.first_time_user) {
						goto('/user/onboarding')
						return
					}
				} catch (err) {
					console.error('Could not fetch global user info for onboarding check:', err)
				}
			}

			if ($workspaceStore) {
				$userStore = await getUserExt($workspaceStore)
				goto(rd ?? '/')
			} else {
				let workspaceTarget = parseQueryParams(rd ?? undefined)['workspace']
				if (rd && workspaceTarget) {
					$workspaceStore = workspaceTarget
					goto(rd)
					return
				}

				if (!$usersWorkspaceStore) {
					try {
						usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
					} catch {}
				}
				const allWorkspaces = $usersWorkspaceStore?.workspaces
				if (allWorkspaces?.length == 1) {
					$workspaceStore = allWorkspaces[0].id
					if (closeUponLogin) {
						closeUponLoginSuccess()
						return
					}
					if (rd) {
						goto(rd, { replaceState: true })
					} else {
						goto('/', { replaceState: true })
					}
					return
				}

				if (closeUponLogin) {
					closeUponLoginSuccess()
					return
				}

				if (rd) {
					goto('/user/workspaces?rd=' + encodeURIComponent(rd), { replaceState: true })
				} else {
					goto('/user/workspaces', { replaceState: true })
				}
			}
		} else {
			if (closeUponLogin) {
				goto('/user/close')
				return
			}
			sendUserToast('Missing code or state as query params', true)
			await logoutWithRedirect(rd ?? undefined)
		}
	})

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')

	if (darkMode === 'dark') {
		document.documentElement.classList.add('dark')
	} else {
		document.documentElement.classList.remove('dark')
	}

	function closeUponLoginSuccess() {
		const message = { type: 'success' }
		if (window.opener) {
			window.opener.postMessage(message, '*')
		} else {
			localStorage.setItem('oauth-success', JSON.stringify(message))
		}
		window.close()
	}
</script>

<CenteredModal title="Login from {clientName}">
	<div class="w-full">
		<div class="block m-auto w-20">
			<WindmillIcon height="80px" width="80px" spin="fast" />
		</div>
	</div>
</CenteredModal>
