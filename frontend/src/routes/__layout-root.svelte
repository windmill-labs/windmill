<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { SvelteToast } from '@zerodevx/svelte-toast'
	import { onMount } from 'svelte'
	import { UserService } from '../gen'
	import { userStore, workspaceStore, type UserExt } from '../stores'
	import { getUser, logout, refreshSuperadmin, sendUserToast } from '../utils'

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

	async function handleRedirections(user?: UserExt, workspace?: string) {
		if (workspace && !user) {
			await UserService.getCurrentEmail()
			$userStore = await getUser(workspace)
			refreshSuperadmin()
		} else {
			if (user) {
				if (workspace) {
					// Default page when logged in
					goto('/scripts')
				} else {
					// Redirect to workspaces when no workspace is selected
					goto('/user/workspaces')
				}
			} else {
				goto('/user/login')
			}
		}
	}

	$: {
		handleRedirections($userStore, $workspaceStore)
	}

	onMount(() => {
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
