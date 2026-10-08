<script lang="ts">
	import { goto } from '$lib/navigation'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { UserService, WorkspaceService } from '$lib/gen'
	import CenteredModal from '$lib/components/CenteredModal.svelte'

	import { userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { logoutWithRedirect } from '$lib/logoutKit'
	import { isValidLogoutRedirect } from '$lib/logoutRedirect'
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
		const rawRd = localStorage.getItem('rd')
		if (rawRd) {
			localStorage.removeItem('rd')
		}
		const rd = rawRd?.startsWith('http') && !isValidLogoutRedirect(rawRd) ? null : rawRd
		const closeUponLogin =
			getCookie('close') == 'true' || localStorage.getItem('closeUponLogin') == 'true'
		// "Finish account setup" sent a signed-in account with no credentials of its own to a
		// provider. Whatever went wrong on the way back — the consent screen cancelled, an
		// address mismatch, an unverified address, a domain rule — that session is the only
		// way into the account, so it must survive: report and go home rather than log out.
		// Read before the backend call, which clears the cookie whether or not it adopts.
		// SAML's ACS answers a top-level POST from the IdP, so a refusal there arrives here
		// as a redirect with the flag in the query.
		const finishingSetup =
			!!getCookie('finish_setup') || page.url.searchParams.get('finish_setup') === '1'
		function backToSetup(message: string) {
			document.cookie = 'finish_setup=; path=/; max-age=0; SameSite=Lax'
			sendUserToast(message, true)
			goto('/')
		}
		if (error) {
			if (finishingSetup) {
				backToSetup(
					error.includes('finish_setup_mismatch')
						? error.replace(/^.*finish_setup_mismatch:\s*/, '')
						: `Signing in with ${clientName} did not go through (${error}). Your account is unchanged.`
				)
				return
			}
			sendUserToast(`Error trying to login with ${clientName} ${error}`, true)
			if (closeUponLogin) {
				closeUponLoginError(`Error trying to login with ${clientName} ${error}`)
				return
			}
			await logoutWithRedirect(rd ?? undefined)
		} else if (code && state && clientName) {
			try {
				await UserService.loginWithOauth({ requestBody: { code, state }, clientName })
			} catch (e) {
				const message = String(e?.body ?? e?.message ?? '')
				if (finishingSetup) {
					backToSetup(message.replace(/^.*finish_setup_mismatch:\s*/, ''))
					return
				}
				if (closeUponLogin) {
					closeUponLoginError(e.body ?? e.message)
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
						// `rd` rides along: someone arriving from a shared hub project signs up with a
						// destination already in hand, and dropping it here strands them in an empty
						// workspace with no sign of what they came to import.
						goto(`/user/onboarding${rd ? `?rd=${encodeURIComponent(rd)}` : ''}`)
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
	// This route bypasses the (root) layout, so restore the variant class too.
	applyDarkModeVariant()

	function closeUponLoginSuccess() {
		relayToOpener({ type: 'success' })
	}

	/** The popup is the only window that saw the server's answer, and it closes: a
	 * refusal that stayed here would leave the page that opened it with nothing to show. */
	function closeUponLoginError(error: string) {
		relayToOpener({ type: 'error', error: typeof error === 'string' ? error : String(error) })
	}

	function relayToOpener(message: { type: 'success' } | { type: 'error'; error: string }) {
		if (window.opener) {
			window.opener.postMessage(message, '*')
		} else {
			localStorage.setItem('oauth-success', JSON.stringify(message))
		}
		window.close()
	}
</script>

<CenteredModal title="Login from {clientName}" loading={true}></CenteredModal>
