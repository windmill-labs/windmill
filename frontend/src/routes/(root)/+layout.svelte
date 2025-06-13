<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { logoutWithRedirect } from '$lib/logout'
	import { userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, onMount } from 'svelte'

	import { refreshSuperadmin } from '$lib/refreshUser'
	// import EditorTheme from '$lib/components/EditorTheme.svelte'
	import { computeDrift } from '$lib/forLater'
	import { setLicense } from '$lib/enterpriseUtils'
	import { deepEqual } from 'fast-equals'
	interface Props {
		children?: import('svelte').Snippet
	}

	let { children }: Props = $props()

	const monacoEditorUnhandledErrors = [
		'Model not found',
		'Connection is disposed.',
		'Connection got disposed.',
		'Stopping the server timed out',
		'Canceled',
		'Starting server failed',
		'Missing service editorService',
		'Unexpected usage',
		'NetworkError when attempting to fetch resource.',
		"Client got disposed and can't be restarted."
	]

	async function setUserWorkspaceStore() {
		$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
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
				if (
					!$page.url.pathname.startsWith('/user/') ||
					$page.url.pathname.startsWith('/user/cli')
				) {
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

	let interval: NodeJS.Timeout | undefined = undefined
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
					message.startsWith('Failed to fetch dynamically imported') ||
					message.startsWith('Unable to figure out browser width and height') ||
					message.startsWith('Unable to read file') ||
					message.startsWith('Could not find source file')
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
		setLicense()
		computeDrift()

		if ($page.url.pathname != '/user/login') {
			setUserWorkspaceStore()
			loadUser()
			UserService.refreshUserToken({ ifExpiringInLessThanS: 60 * 60 * 24 })
		}

		let i = 0
		interval = setInterval(async () => {
			if ($page.url.pathname != '/user/login') {
				i += 1

				// every 1 hour
				if (i % 12 == 0) {
					console.debug('Refreshing user token')
					await UserService.refreshUserToken({ ifExpiringInLessThanS: 60 * 60 * 24 })
				}

				try {
					const workspace = $workspaceStore
					const user = $userStore

					if (workspace && user) {
						const newUser = await getUserExt(workspace)
						if (!deepEqual(newUser, $userStore)) {
							userStore.set(newUser)
							console.info('refreshed user')
						} else {
							console.debug('user is the same')
						}
					}
				} catch (e) {
					console.error('Could not refresh user', e)
				}
			}
			// every 5 minues
		}, 300000)
	})

	onDestroy(() => {
		interval && clearInterval(interval)
	})

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')

	if (darkMode === 'dark') {
		document.documentElement.classList.add('dark')
	} else {
		document.documentElement.classList.remove('dark')
	}
</script>

{@render children?.()}
