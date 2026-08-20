<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'

	import { UserService, WorkspaceService } from '$lib/gen'
	import {
		usersWorkspaceStore,
		workspaceStore,
		userStore,
		enterpriseLicense,
		whitelabelNameStore
	} from '$lib/stores'
	import { emptyString, parseQueryParams } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { WindmillIcon } from '$lib/components/icons'
	import LoginPageHeader from '$lib/components/LoginPageHeader.svelte'
	import { clearStores } from '$lib/storeUtils'
	import { setLicense } from '$lib/enterpriseUtils'
	import Login from '$lib/components/Login.svelte'
	import LoginHeading from '$lib/components/LoginHeading.svelte'
	import { onMount } from 'svelte'
	import { refreshSuperadmin } from '$lib/refreshUser'
	import { isValidLogoutRedirect, toSameOriginRelativePath } from '$lib/logoutRedirect'

	const email = page.url.searchParams.get('email') ?? ''
	const password = page.url.searchParams.get('password') ?? ''
	const error = page.url.searchParams.get('error') ?? undefined
	const rdFromStorage = localStorage.getItem('rd') || undefined
	if (rdFromStorage) {
		localStorage.removeItem('rd')
	}
	const rawRd = page.url.searchParams.get('rd') ?? rdFromStorage
	// Reduce same-origin full URLs (e.g. the page URL persisted from
	// PublicApp.svelte's /a/[...path]) to relative paths so the post-login
	// redirect can honor them. Without this, rd falls into the
	// `startsWith('http')` branch and gets bounced to '/' (or worse, hung).
	const sameOriginRd = toSameOriginRelativePath(rawRd)
	const rd = sameOriginRd ?? rawRd

	let firstTime = $state(false)
	// A third-party login creates the account on first use, so the page only offers sign-up
	// once the instance has one configured. undefined until the card reports what it loaded.
	let hasThirdParty = $state<boolean | undefined>(undefined)

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
			if (isValidLogoutRedirect(rd)) {
				window.location.href = rd
				return
			}
			goto('/')
			return
		}

		try {
			if (!$usersWorkspaceStore) {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			}
			await refreshSuperadmin()
		} catch {}

		if ($workspaceStore) {
			goto(rd ?? '/')
		} else {
			let workspaceTarget = parseQueryParams(rd ?? undefined)['workspace']
			if (rd && workspaceTarget) {
				$workspaceStore = workspaceTarget
				goto(rd)
				return
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
						const prefix = defaultApp.default_app_raw ? '/apps_raw/get' : '/apps/get'
						goto(`${prefix}/${defaultApp.default_app_path}`)
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

<!-- min-h-screen, not h-screen: a card taller than the viewport (many providers, one per row)
	would otherwise overflow a centered fixed-height box on both edges, leaving the top of the
	card unreachable since the page can only scroll down. -->
<div
	class="flex flex-col justify-center py-12 sm:px-6 lg:px-8 relative bg-surface-secondary min-h-screen"
>
	<LoginPageHeader />
	<div class="sm:mx-auto sm:w-full sm:max-w-sm">
		<div class="mx-auto flex justify-center">
			{#if !$enterpriseLicense || !$whitelabelNameStore}
				<WindmillIcon height="48px" width="48px" spin="slow" />
			{/if}
		</div>
		<div class="mt-6">
			<LoginHeading {hasThirdParty} />
		</div>
	</div>

	<div class="mt-6 sm:mx-auto sm:w-full sm:max-w-sm mb-48">
		<Login
			{firstTime}
			{rd}
			{error}
			{password}
			{email}
			autoRedirect={false}
			onOptionsLoaded={(options) => (hasThirdParty = options.hasThirdParty)}
		/>
	</div>
</div>
