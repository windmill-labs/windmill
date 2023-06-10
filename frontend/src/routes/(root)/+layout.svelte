<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { OpenAPI, SettingsService, UserService, WorkspaceService } from '$lib/gen'
	import { logoutWithRedirect } from '$lib/logout'
	import { enterpriseLicense, userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import github from 'svelte-highlight/styles/github'
	import { refreshSuperadmin } from '$lib/refreshUser'

	let token = $page.url.searchParams.get('wm_token') ?? undefined
	if (token) {
		OpenAPI.WITH_CREDENTIALS = true
		OpenAPI.TOKEN = $page.url.searchParams.get('wm_token')!
	}

	const monacoEditorUnhandledErrors = [
		'Model not found',
		'Connection is disposed.',
		'Connection got disposed.',
		'Stopping the server timed out',
		'Canceled',
		'Missing service editorService',
		'Unexpected usage',
		'NetworkError when attempting to fetch resource.'
	]

	async function setUserWorkspaceStore() {
		$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
	}

	async function setLicense() {
		const license = await SettingsService.getLicenseId()
		if (license) {
			$enterpriseLicense = license
		}
	}

	async function loadUser() {
		try {
			await refreshSuperadmin()

			if ($workspaceStore) {
				if ($userStore) {
					console.log(`Welcome back ${$userStore.username} to ${$workspaceStore}`)
				} else {
					$userStore = await getUserExt($workspaceStore)
					if (!$userStore) {
						throw Error('Not logged in')
					}
				}
			} else {
				if (!$page.url.pathname.startsWith('/user/')) {
					goto(
						`/user/workspaces?rd=${encodeURIComponent(
							$page.url.href.replace($page.url.origin, '')
						)}`
					)
				}
				let user = await UserService.globalWhoami()
				console.log(`Welcome back ${user.email}`)
			}
		} catch (e) {
			console.error(e)
			if ($page.url.pathname != '/user/login') {
				const url = $page.url
				await logoutWithRedirect(url.href.replace(url.origin, ''))
			}
		}
	}

	onMount(() => {
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()

			if (event.reason?.message) {
				const { message, body, status } = event.reason

				if (message === 'Missing service editorService') {
					console.error('Reloading the page to fix a Monaco Editor bug')
					location.reload()
					return
				}
				// Unhandled errors from Monaco Editor don't logout the user
				if (
					monacoEditorUnhandledErrors.includes(message) ||
					message.startsWith('Failed to fetch dynamically imported')
				) {
					console.warn(message)
					return
				}
				if (message == 'Client not running') {
					sendUserToast(
						'Unrecoverable error for the smart assistant. Refresh the page to get the full experience again (This issue is WIP and will get fixed)',
						true
					)
					return
				}

				if (status == '401') {
					const url = $page.url
					console.log('UNAUTHORIZED', url, url.href.replace(url.origin, ''))
					if (url.pathname != '/user/login' && url.pathname != '/user/logout') {
						logoutWithRedirect(url.href.replace(url.origin, ''))
						return
					}
				} else if (status == '403') {
					sendUserToast('An endpoint required a privilege which you do not have', true)
				} else {
					if (body) {
						sendUserToast(`${body}`, true)
					} else {
						sendUserToast(`${message}`, true)
					}
				}
			} else {
				console.error('Caught unhandled promise rejection without message', event.reason, event)
			}
		}
		if ($page.url.pathname != '/user/login') {
			setUserWorkspaceStore()
			loadUser()
			setLicense()
		}
	})
</script>

<svelte:head>
	{@html github}
</svelte:head>

<slot />
