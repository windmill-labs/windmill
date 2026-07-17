<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { logoutWithRedirect } from '$lib/logoutKit'
	import {
		clearWorkspaceFromStorage,
		superadmin,
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

	// A fork that was deleted remotely while the tab was open disappears from the
	// user's workspace list on the next load (the list is membership-gated and the
	// row is hard-deleted). Landing on it otherwise breaks the page: a member gets
	// logged out (whoami fails), a superadmin silently renders a dead workspace
	// whose every request 404s. Detect the vanished fork and bounce the user to its
	// parent (remembered in localStorage while the fork was reachable), or to the
	// workspace picker if the parent is unknown / also gone. Returns true when it
	// handled the situation so the caller skips normal loading.
	async function tryRecoverFromDeletedFork(
		workspaceId: string,
		workspacesPromise: Promise<{ workspaces: { id: string }[] }>
	): Promise<boolean> {
		// Managed dev workspaces are forks with bare IDs (no `wm-fork-` prefix), so
		// the prefix alone can't identify every fork. A remembered parent — recorded
		// while the workspace was still reachable and its parent_workspace_id visible
		// — flags those prefixless forks once their server row is gone.
		const looksLikeFork =
			workspaceId.startsWith('wm-fork-') || getRememberedForkParent(workspaceId) != undefined
		if (!looksLikeFork) return false

		// Reuse the list already being fetched in onMount rather than issuing a
		// second identical request.
		let workspaces: { id: string }[]
		try {
			workspaces = (await workspacesPromise).workspaces
		} catch {
			return false
		}

		// Fork still reachable → nothing to recover from.
		if (workspaces.some((w) => w.id === workspaceId)) return false

		// The membership-gated list omits any workspace the user isn't a member of,
		// so a superadmin's absence from it doesn't mean the fork is gone. Confirm
		// genuine deletion against the DB before recovering — otherwise a live fork
		// the superadmin merely isn't a member of would be treated as deleted.
		if ($superadmin) {
			try {
				await WorkspaceService.getWorkspaceAsSuperAdmin({ workspace: workspaceId })
				// Fork still exists → not deleted, let normal loading proceed.
				return false
			} catch (e) {
				// Only a 404 confirms the fork is truly gone. Any other failure
				// (transient network error, 500, …) is inconclusive, so abort the
				// deletion path rather than redirect away from a live workspace.
				if ((e as { status?: number } | null | undefined)?.status !== 404) {
					return false
				}
			}
		}

		// The fork is gone, but only redirect if the session itself is still valid;
		// otherwise let the normal flow handle the (genuine) auth failure.
		try {
			await UserService.globalWhoami()
		} catch {
			return false
		}

		const parentId = getRememberedForkParent(workspaceId)
		const parentReachable = parentId != undefined && workspaces.some((w) => w.id === parentId)
		forgetForkParent(workspaceId)

		if (parentId != undefined && parentReachable) {
			switchWorkspace(parentId)
			const parentUser = await getUserExt(parentId)
			if (parentUser) {
				$userStore = parentUser
				sendUserToast(
					`Fork ${workspaceId} was deleted remotely, returning you to its parent workspace ${parentId}.`,
					'warning'
				)
				await goto('/')
				return true
			}
			// Parent unexpectedly didn't resolve a user — fall through to the picker
			// rather than risk the forced-logout path this recovery exists to avoid.
		}

		clearWorkspaceFromStorage()
		workspaceStore.set(undefined)
		sendUserToast(`Fork ${workspaceId} is no longer available, please pick a workspace.`, 'warning')
		await goto('/user/workspaces')
		return true
	}

	async function loadUser(workspacesPromise: Promise<{ workspaces: { id: string }[] }>) {
		try {
			await refreshSuperadmin()

			if ($workspaceStore) {
				// Check up-front (independent of whoami, which succeeds for superadmins
				// even on a dead workspace) whether the active fork was deleted remotely.
				if (await tryRecoverFromDeletedFork($workspaceStore, workspacesPromise)) {
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
			loadUser(setUserWorkspaceStore())
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
</script>

{@render children?.()}
