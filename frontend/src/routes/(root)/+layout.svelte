<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { logoutWithRedirect } from '$lib/logoutKit'
	import {
		clearWorkspaceFromStorage,
		userStore,
		usersWorkspaceStore,
		workspaceStore
	} from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { forgetForkParent, getRememberedForkParent } from '$lib/forkParentMemory'
	import { onDestroy, onMount } from 'svelte'

	import { refreshSuperadmin } from '$lib/refreshUser'
	// import EditorTheme from '$lib/components/EditorTheme.svelte'
	import { computeDrift } from '$lib/forLater'
	import { setLicense } from '$lib/enterpriseUtils'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
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
		const list = await WorkspaceService.listUserWorkspaces()
		$usersWorkspaceStore = list
		return list
	}

	// A fork deleted remotely while the tab was open leaves the client pointing at a
	// dead workspace id: every request 404s and members get logged out. The fork's
	// parent linkage lived only in its own (now deleted) row, so we mirror it to
	// localStorage while the fork is reachable (see forkParentMemory) and use it here
	// to send the user back to the parent. Returns true when it handled the situation
	// so the caller skips normal loading.
	async function tryRecoverFromDeletedWorkspace(workspaceId: string): Promise<boolean> {
		// Only forks are recoverable, identified two ways: the `wm-fork-` id prefix, or
		// a remembered parent. Neither alone is sufficient — dev-workspace forks carry
		// no prefix (only a remembered parent), while a non-member superadmin's fork has
		// the prefix but no remembered parent (the recorder only sees the user's own
		// membership-gated list). A workspace that is neither is left to normal loading.
		const parentId = getRememberedForkParent(workspaceId)
		if (parentId == undefined && !workspaceId.startsWith('wm-fork-')) return false

		// `exists` is not membership-gated, so it settles "is this workspace gone?"
		// identically for members, non-members and superadmins, and it requires a valid
		// session. Only a conclusive `false` means deleted: any rejection (expired
		// session, transient failure) is inconclusive and must leave the normal loading
		// path — including its genuine auth handling — untouched.
		let exists: boolean
		try {
			exists = await WorkspaceService.existsWorkspace({ requestBody: { id: workspaceId } })
		} catch {
			return false
		}
		if (exists) return false

		forgetForkParent(workspaceId)

		// Send the user to the remembered parent when we have one; otherwise (unknown or
		// inaccessible parent) fall back to the picker rather than the forced-logout path
		// this recovery exists to avoid.
		if (parentId != undefined) {
			switchWorkspace(parentId)
			const parentUser = await getUserExt(parentId)
			if (parentUser) {
				$userStore = parentUser
				sendUserToast(
					`Workspace ${workspaceId} not found, switched to parent workspace ${parentId}.`,
					'warning'
				)
				await goto('/')
				return true
			}
		}

		try {
			clearWorkspaceFromStorage()
		} catch (e) {
			console.error('Could not clear workspace storage during deleted-workspace recovery', e)
		}
		workspaceStore.set(undefined)
		sendUserToast(
			`Workspace ${workspaceId} is no longer available, please pick a workspace.`,
			'warning'
		)
		await goto('/user/workspaces')
		return true
	}

	async function loadUser() {
		try {
			await refreshSuperadmin()

			if ($workspaceStore) {
				if (await tryRecoverFromDeletedWorkspace($workspaceStore)) {
					return
				}
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
					(!page.url.pathname.startsWith('/user/') || page.url.pathname.startsWith('/user/cli')) &&
					!page.url.pathname.startsWith('/oauth/mcp_authorize')
				) {
					goto(
						`/user/workspaces?rd=${encodeURIComponent(page.url.href.replace(page.url.origin, ''))}`
					)
				}
				let user = await UserService.globalWhoami()
				console.log(`Welcome back ${user.email}`)
			}
		} catch (e) {
			console.error(e)
			if (page.url.pathname != '/user/login' && page.url.pathname != '/user/logout') {
				const url = page.url
				console.log('logout 5', url.href.replace(url.origin, ''))
				await logoutWithRedirect(url.href.replace(url.origin, ''))
			}
		}
	}

	let interval: number | undefined = undefined
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
					const url = page.url
					console.log('UNAUTHORIZED', url, url.href.replace(url.origin, ''), url.pathname)
					if (url.pathname != '/user/login' && url.pathname != '/user/logout') {
						console.log('logout 6', url.pathname, url.href.replace(url.origin, ''))
						logoutWithRedirect(url.href.replace(url.origin, ''))
						return
					} else {
						console.log('logout ignored')
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

		if (page.url.pathname != '/user/login') {
			setUserWorkspaceStore().catch((e) => console.error('could not load workspace list', e))
			loadUser()
			UserService.refreshUserToken({ ifExpiringInLessThanS: 30 * 60 })
		}

		let i = 0
		interval = setInterval(async () => {
			if (page.url.pathname != '/user/login') {
				i += 1

				// every 15 mins
				if (i % 3 == 0) {
					console.debug('Refreshing user token')
					// every 15 minutes check if session token is expiring in less than 30min
					await UserService.refreshUserToken({ ifExpiringInLessThanS: 30 * 60 })
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

	applyDarkModeVariant()
</script>

{@render children?.()}
