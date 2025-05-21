<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/stores'

	import { UserService, WorkspaceService } from '$lib/gen'
	import {
		usersWorkspaceStore,
		workspaceStore,
		userStore,
		enterpriseLicense,
		whitelabelNameStore
	} from '$lib/stores'
	import { classNames, emptyString, parseQueryParams } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { WindmillIcon } from '$lib/components/icons'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { clearStores } from '$lib/storeUtils'
	import { setLicense } from '$lib/enterpriseUtils'
	import Login from '$lib/components/Login.svelte'
	import { onMount } from 'svelte'

	const email = $page.url.searchParams.get('email') ?? ''
	const password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd') ?? undefined

	let showPassword = false
	let firstTime = false

	function clearWindmillCloudCookies() {
		const domain = window.location.hostname
		// Check if the domain ends with ".windmill.dev" but is NOT "app.windmill.dev"
		if (
			domain.endsWith('.windmill.dev') &&
			domain !== 'app.windmill.dev' &&
			domain !== 'internal.windmill.dev'
		) {
			// Remove the "token" cookie for the current domain and its parent domain
			document.cookie = `token=; domain=.windmill.dev; path=/; expires=Thu, 01 Jan 1970 00:00:00 UTC; Secure; SameSite=None`
			document.cookie = `token=; path=/; expires=Thu, 01 Jan 1970 00:00:00 UTC; Secure; SameSite=None`

			console.log('Token cookie removed for windmill cloud instance.')
		}
	}

	onMount(() => {
		clearWindmillCloudCookies()
	})

	async function redirectUser() {
		if (rd?.startsWith('http')) {
			window.location.href = rd
			return
		}
		if ($workspaceStore) {
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

			const allWorkspaces = $usersWorkspaceStore?.workspaces.filter((x) => x.id != 'admins')

			if (allWorkspaces?.length == 1) {
				workspaceStore.set(allWorkspaces[0].id)
				$userStore = await getUserExt($workspaceStore!)

				if (!$userStore?.is_super_admin && $userStore?.operator) {
					let defaultApp = await WorkspaceService.getWorkspaceDefaultApp({
						workspace: $workspaceStore!
					})
					if (!emptyString(defaultApp.default_app_path)) {
						goto(`/apps/get/${defaultApp.default_app_path}`)
					} else {
						goto(rd ?? '/')
					}
				} else {
					goto(rd ?? '/')
				}
			} else if (rd?.startsWith('/user/workspaces')) {
				goto(rd)
			} else if (rd == '/#user-settings') {
				goto(`/user/workspaces#user-settings`)
			} else {
				goto(`/user/workspaces${rd ? `?rd=${encodeURIComponent(rd)}` : ''}`)
			}
		}
	}

	async function redirectIfNecessary() {
		await UserService.getCurrentEmail()
		redirectUser()
	}

	async function checkFirstTimeSetup() {
		firstTime = await (await fetch('/api/auth/is_first_time_setup')).json()
	}

	try {
		setLicense()
		redirectIfNecessary()
		checkFirstTimeSetup()
	} catch {
		clearStores()
	}
</script>

<div
	class="flex flex-col justify-center py-12 sm:px-6 lg:px-8 relative bg-surface-secondary h-screen"
>
	<LoginPageHeader />
	<div class="sm:mx-auto sm:w-full sm:max-w-md">
		<div class="mx-auto flex justify-center">
			{#if !$enterpriseLicense || !$whitelabelNameStore}
				<WindmillIcon height="80px" width="80px" spin="slow" />
			{/if}
		</div>
		<h2 class="mt-6 text-center text-3xl font-bold tracking-tight text-primary">
			Log in or sign up
		</h2>
		<p class="mt-2 text-center text-sm text-secondary">
			Log in or sign up with any of the methods below
		</p>
	</div>

	<div
		class={classNames('mt-8 sm:mx-auto sm:w-full sm:max-w-xl', showPassword ? 'mb-16' : 'mb-48')}
	>
		<div class="flex justify-end">
			<DarkModeToggle forcedDarkMode={false} />
		</div>
		<Login {firstTime} {rd} {error} {password} {email} />
	</div>
</div>
