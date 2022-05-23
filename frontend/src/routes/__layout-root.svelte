<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { SvelteToast } from '@zerodevx/svelte-toast'
	import { onMount } from 'svelte'
	import { UserService, WorkspaceService } from '../gen'
	import {
		clearStores,
		superadmin,
		usernameStore,
		usersWorkspaceStore,
		workspaceStore
	} from '../stores'
	import { getUser, logout, logoutWithRedirect, refreshSuperadmin, sendUserToast } from '../utils'

	// Default toast options
	const toastOptions = {
		duration: 4000, // duration of progress bar tween to the `next` value
		initial: 1, // initial progress bar value
		next: 0, // next progress value
		pausable: false, // pause progress bar tween on mouse hover
		dismissable: true, // allow dismiss with close button
		reversed: false, // insert new toast to bottom of stack
		intro: { x: 256 }, // toast intro fly animation settings
		theme: {} // css var overrides
	}

	const monacoEditorUnhandledErrors = [
		'Model not found',
		'Connection is disposed.',
		'Connection got disposed.'
	]

	async function redirectIfLoggedIn(): Promise<void> {
		try {
			await UserService.getCurrentEmail()
			goto('/')
		} catch {
			clearStores()
		}
	}

	async function loadData() {
		try {
			$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
			await refreshSuperadmin()

			if ($workspaceStore && $usernameStore) {
				await getUser($workspaceStore)
			} else if ($superadmin) {
				console.log('You are a superadmin, you can go wherever you please')
			} else {
				goto('/user/workspaces')
			}
		} catch {
			logoutWithRedirect($page.url.pathname)
		}
	}

	async function handleRedirectionsAndLoadData() {
		if ($page.url.pathname === '/user/login') {
			redirectIfLoggedIn()
		} else {
			loadData()
		}
	}

	onMount(() => {
		handleRedirectionsAndLoadData()

		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()

			if (event.reason?.message) {
				const { message, body, status } = event.reason

				// Unhandled errors from Monaco Editor don't logout the user
				if (monacoEditorUnhandledErrors.includes(message)) {
					return
				}

				if (status == '401') {
					sendUserToast('Logged out after a request was unauthorized', true)
					logout($page.url.pathname)
				} else {
					sendUserToast(`${message}: ${body ?? ''}`, true)
				}
			} else {
				console.log('Caught unhandled promise rejection without message', event)
			}
		}
	})
</script>

<slot />
<SvelteToast {toastOptions} />

<style>
	:root {
		--toastBackground: #eff6ff;
		--toastBarBackground: #eff6ff;
		--toastColor: #123456;
	}
</style>
