<script lang="ts">
	import { goto } from '$lib/navigation'
	import Github from '$lib/components/icons/brands/Github.svelte'
	import Gitlab from '$lib/components/icons/brands/Gitlab.svelte'
	import Google from '$lib/components/icons/brands/Google.svelte'
	import Microsoft from '$lib/components/icons/brands/Microsoft.svelte'
	import Okta from '$lib/components/icons/brands/Okta.svelte'
	import Auth0 from '$lib/components/icons/brands/Auth0.svelte'
	import NextcloudIcon from '$lib/components/icons/NextcloudIcon.svelte'
	import PocketIdIcon from '$lib/components/icons/PocketIdIcon.svelte'

	import { OauthService, UserService, WorkspaceService } from '$lib/gen'
	import { usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { classNames, emptyString, escapeHtml, parseQueryParams } from '$lib/utils'
	import { base } from '$lib/base'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { refreshSuperadmin } from '$lib/refreshUser'
	import { onDestroy, onMount } from 'svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Button from './common/button/Button.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'

	interface Props {
		rd?: string | undefined
		email?: string | undefined
		password?: string | undefined
		error?: string | undefined
		popup?: boolean
		firstTime?: boolean
		onLoginSuccess?: () => void
	}

	let {
		rd = undefined,
		email = $bindable(undefined),
		password = $bindable(undefined),
		error = undefined,
		popup = false,
		firstTime = false,
		onLoginSuccess = undefined
	}: Props = $props()

	const providers = [
		{
			type: 'github',
			name: 'GitHub',
			icon: Github
		},
		{
			type: 'gitlab',
			name: 'GitLab',
			icon: Gitlab
		},
		{
			type: 'google',
			name: 'Google',
			icon: Google
		},
		{
			type: 'microsoft',
			name: 'Microsoft',
			icon: Microsoft
		},
		{
			type: 'okta',
			name: 'Okta',
			icon: Okta
		},
		{
			type: 'auth0',
			name: 'Auth0',
			icon: Auth0
		},
		{
			type: 'nextcloud',
			name: 'Nextcloud',
			icon: NextcloudIcon
		},
		{
			type: 'pocketid',
			name: 'Pocket ID',
			icon: PocketIdIcon
		}
	] as const

	const providersType = providers.map((p) => p.type as string)

	let showPassword = $state(false)
	let logins: OAuthLogin[] | undefined = $state(undefined)
	let saml: string | undefined = $state(undefined)
	let smtpConfigured: boolean | undefined = $state(undefined)

	type OAuthLogin = {
		type: string
		displayName: string
	}

	async function login(): Promise<void> {
		if (!email || !password) {
			sendUserToast('Please fill in both email and password', true)
			return
		}

		const requestBody = {
			email,
			password
		}

		try {
			await UserService.login({ requestBody })
		} catch (err) {
			sendUserToast('Invalid credentials', true)
			return
		}

		if (firstTime) {
			goto('/user/first-time')
			return
		}

		// Once logged in, we can fetch the workspaces
		$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
		// trigger a reload of the user
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}

		// Finally, we check whether the user is a superadmin
		refreshSuperadmin()
		redirectUser()
	}

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

	async function loadLogins() {
		try {
			const allLogins = await OauthService.listOauthLogins()
			logins = allLogins.oauth.map((login) => ({
				type: login.type,
				displayName: login.display_name || login.type
			}))
			saml = allLogins.saml

			showPassword = (logins.length == 0 && !saml) || (email != undefined && email.length > 0)
		} catch (e) {
			logins = []
			saml = undefined
			showPassword = true
			console.error('Could not load logins', e)
		}
	}

	loadLogins()

	$effect(() => {
		if (firstTime && !email && !password) {
			email = 'admin@windmill.dev'
			password = 'changeme'
		}
	})

	async function checkSmtpConfigured() {
		try {
			smtpConfigured = await UserService.isSmtpConfigured()
		} catch (err) {
			console.error('Could not check if SMTP is configured', err)
			smtpConfigured = false
		}
	}

	checkSmtpConfigured()

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			login()
		}
	}

	onMount(() => {
		try {
			localStorage.removeItem('closeUponLogin')
		} catch (e) {
			console.error('Could not remove closeUponLogin from local storage', e)
		}
	})

	function popupListener(event) {
		let data = event.data
		// console.log('popupListener', data, event.origin, window.location.origin)
		if (!sameTopDomainOrigin(event.origin, window.location.origin)) {
			console.log('popupListener from different origin', event.origin, window.location.origin)
			return
		}

		processPopupData(data)
		if (data.type === 'success' || data.type === 'error') {
			console.log('Removing popup listener')
			window.removeEventListener('message', popupListener)
		}
	}

	function processPopupData(data) {
		if (data.type === 'error') {
			sendUserToast(data.error, true)
		} else if (data.type === 'success') {
			onLoginSuccess?.()
		}
	}

	function handleStorageEvent(event) {
		if (event.key === 'oauth-success') {
			try {
				processPopupData(JSON.parse(event.newValue))
				console.log('oauth-success from storage')
				// Clean up
				localStorage.removeItem('oauth-success')
				window.removeEventListener('storage', handleStorageEvent)
			} catch (e) {
				console.error('Could not process oauth-success from storage', e)
			}
		} else {
			console.log('Storage event', event.key)
		}
	}

	onDestroy(() => {
		window.removeEventListener('message', popupListener)
		window.removeEventListener('storage', handleStorageEvent)
	})

	function storeRedirect(provider: string) {
		if (rd) {
			try {
				localStorage.setItem('rd', rd)
			} catch (e) {
				console.error('Could not persist redirection to local storage', e)
			}
		}
		let url = base + '/api/oauth/login/' + provider + (popup ? '?close=true' : '')
		console.log('storeRedirect', popup, url)

		if (popup) {
			localStorage.setItem('closeUponLogin', 'true')
			window.addEventListener('message', popupListener)
			window.addEventListener('storage', handleStorageEvent)
			window.open(url, '_blank', 'popup')
		} else {
			localStorage.setItem('closeUponLogin', 'false')
			window.location.href = url
		}
	}

	$effect(() => {
		error && sendUserToast(escapeHtml(error), true)
	})
</script>

<div class="bg-surface px-4 py-8 border sm:rounded-lg sm:px-10">
	<div class="grid {logins && logins.length > 2 ? 'grid-cols-2' : ''} gap-4">
		{#if !logins}
			{#each Array(4) as _}
				<Skeleton layout={[0.5, [2.375]]} />
			{/each}
		{:else}
			{#each providers as { type, icon }}
				{#if logins?.some((login) => login.type === type)}
					<Button
						variant="default"
						startIcon={{ icon, classes: 'h-4' }}
						on:click={() => storeRedirect(type)}
					>
						{logins.find((login) => login.type === type)?.displayName}
					</Button>
				{/if}
			{/each}
			{#each logins.filter((login) => !providersType?.includes(login.type)) as login}
				<Button
					variant="default"
					btnClasses="mt-2 w-full"
					on:click={() => storeRedirect(login.type)}
				>
					{login.displayName}
				</Button>
			{/each}
		{/if}
		{#if saml}
			<Button
				variant="default"
				btnClasses="mt-2 w-full"
				on:click={() => {
					if (saml) {
						window.location.href = saml
					} else {
						sendUserToast('No SAML login available', true)
					}
				}}
			>
				SSO
			</Button>
		{/if}
	</div>
	{#if saml || (logins && logins.length > 0)}
		<div class={classNames('center-center', logins && logins.length > 0 ? 'mt-6' : '')}>
			<Button
				size="xs"
				variant="subtle"
				on:click={() => {
					showPassword = !showPassword
				}}
			>
				Log in without third-party
			</Button>
		</div>
	{/if}

	{#if showPassword}
		<div>
			{#if firstTime}
				<p class="text-xs text-center w-full pb-4 text-secondary">
					Welcome! Default credentials admin@windmill.dev / changeme have been prefilled.
				</p>
			{/if}
			<div class="space-y-6">
				{#if isCloudHosted()}
					<p class="text-xs text-secondary pb-6">
						To get credentials without the OAuth providers above, send an email at
						contact@windmill.dev
					</p>
				{/if}
				<div class="space-y-1">
					<label for="email" class="block text-xs font-semibold text-emphasis"> Email </label>
					<div>
						<input type="email" bind:value={email} id="email" autocomplete="email" />
					</div>
				</div>

				<div class="space-y-1">
					<label for="password" class="block text-xs font-semibold text-emphasis"> Password </label>
					<div>
						<input
							onkeyup={handleKeyUp}
							bind:value={password}
							id="password"
							type="password"
							autocomplete="current-password"
						/>
					</div>
					{#if smtpConfigured}
						<div class="text-right pt-1">
							<a
								href="{base}/user/forgot-password"
								class="text-2xs text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
							>
								Forgot password?
							</a>
						</div>
					{/if}
				</div>

				<div class="pt-2">
					<Button onClick={login} variant="accent" disabled={!email || !password}>Sign in</Button>
				</div>
			</div>

			{#if isCloudHosted()}
				<p class="text-2xs text-secondary mt-10 text-center">
					By logging in, you agree to our
					<a href="https://windmill.dev/terms_of_service" target="_blank" rel="noreferrer">
						Terms of service
					</a>
					and
					<a href="https://windmill.dev/privacy_policy" target="_blank" rel="noreferrer">
						Privacy policy
					</a>
				</p>
			{/if}
		</div>
	{/if}
</div>
