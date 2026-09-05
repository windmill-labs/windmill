<script module lang="ts">
	import type { LastLoginMethod } from '$lib/lastLoginMethod'

	/** Feeds the login card a fixed instance configuration instead of the live one.
	 * Only the kitchen sink at /kitchen_sink/login sets it; production always fetches. */
	export type LoginPreview = {
		logins?: { type: string; displayName: string }[]
		saml?: boolean
		disablePasswordLogin?: boolean
		smtpConfigured?: boolean
		cloud?: boolean
		autoRedirecting?: boolean
		lastUsed?: LastLoginMethod
	}
</script>

<script lang="ts">
	import { goto } from '$lib/navigation'
	import {
		Auth0Icon,
		GithubIcon,
		GitlabIcon,
		GoogleIcon,
		MicrosoftIcon,
		NextcloudIcon,
		OktaIcon
	} from '$lib/components/icons'
	import PocketIdIcon from '$lib/components/icons/PocketIdIcon.svelte'

	import { OauthService, UserService, WorkspaceService } from '$lib/gen'
	import { usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { emptyString, escapeHtml, parseQueryParams } from '$lib/utils'
	import { base } from '$lib/base'
	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { refreshSuperadmin } from '$lib/refreshUser'
	import { onDestroy, onMount, tick } from 'svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Button from './common/button/Button.svelte'
	import Password from './Password.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { sameTopDomainOrigin } from '$lib/cookies'
	import { isValidLogoutRedirect, toSameOriginRelativePath } from '$lib/logoutRedirect'
	import InputError from './InputError.svelte'
	import { loginErrorMessage } from '$lib/loginError'
	import Badge from './common/badge/Badge.svelte'
	import {
		getLastLoginMethod,
		clearPendingLoginMethod,
		confirmPendingLoginMethod,
		markLoginMethodPending,
		rememberLoginMethod,
		sameLoginMethod
	} from '$lib/lastLoginMethod'

	interface Props {
		rd?: string | undefined
		email?: string | undefined
		password?: string | undefined
		error?: string | undefined
		popup?: boolean
		firstTime?: boolean
		autoRedirect?: boolean
		onLoginSuccess?: () => void
		/** A refusal the popup relayed back, in the server's words. */
		onLoginError?: (message: string) => void
		preview?: LoginPreview
		/** Reports the instance's login options once loaded, so the page around the card can
		 * adapt its heading: a third-party login also creates the account on first use. */
		onOptionsLoaded?: (options: { hasThirdParty: boolean }) => void
		/** `<workspace>/<app_path>` when this sign-in is someone opening an app that is
		 * open to guests. A third-party login then mints a guest session -- no account,
		 * no seat -- instead of creating a user. Omitting it is what promotion is: the
		 * same sign-in without this, which provisions them for real. Password login
		 * ignores it: a guest has no stored credential to check. */
		guestApp?: string | undefined
	}

	let {
		rd = undefined,
		email = $bindable(undefined),
		password = $bindable(undefined),
		error = undefined,
		popup = false,
		firstTime = false,
		autoRedirect = true,
		onLoginSuccess = undefined,
		onLoginError = undefined,
		preview = undefined,
		onOptionsLoaded = undefined,
		guestApp = undefined
	}: Props = $props()

	// The harness never takes effect in a production bundle, whatever a caller passes.
	let previewConfig = $derived(import.meta.env.DEV ? preview : undefined)
	let cloudHosted = $derived(previewConfig ? !!previewConfig.cloud : isCloudHosted())

	let lastUsed = $state<LastLoginMethod | undefined>(undefined)
	let lastUsedPassword = $derived(!!lastUsed && lastUsed.kind === 'password')

	// Scoped per instance: the kitchen sink mounts every card at once, and a hardcoded id
	// would point each card's labels and aria-describedby at the first card's fields.
	const uid = $props.id()
	const emailId = `${uid}-email`
	const passwordId = `${uid}-password`
	const errorId = `${uid}-error`
	const emailErrorId = `${uid}-email-error`

	const providers = [
		{
			type: 'github',
			name: 'GitHub',
			icon: GithubIcon
		},
		{
			type: 'gitlab',
			name: 'GitLab',
			icon: GitlabIcon
		},
		{
			type: 'google',
			name: 'Google',
			icon: GoogleIcon
		},
		{
			type: 'microsoft',
			name: 'Microsoft',
			icon: MicrosoftIcon
		},
		{
			type: 'okta',
			name: 'Okta',
			icon: OktaIcon
		},
		{
			type: 'auth0',
			name: 'Auth0',
			icon: Auth0Icon
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

	type ThirdPartyMethod = {
		method: { kind: 'oauth'; provider: string } | { kind: 'saml' }
		displayName: string
		icon?: any
	}

	// rank() maps an unknown type to known.length, not indexOf's -1, so a custom OAuth client
	// sorts after the known providers rather than ahead of all of them. SAML sits at the end,
	// and whatever worked last time is hoisted to the front.
	let orderedThirdParty = $derived.by(() => {
		const known = providers.map((p) => p.type as string)
		const rank = (type: string) => (known.indexOf(type) === -1 ? known.length : known.indexOf(type))
		const oauth: ThirdPartyMethod[] = [...(logins ?? [])]
			.sort((a, b) => rank(a.type) - rank(b.type))
			.map((login) => ({
				method: { kind: 'oauth', provider: login.type },
				displayName: login.displayName,
				icon: providers.find((p) => p.type === login.type)?.icon
			}))
		const all: ThirdPartyMethod[] = saml
			? [...oauth, { method: { kind: 'saml' }, displayName: 'SSO' }]
			: oauth
		const lastIdx = all.findIndex((m) => sameLoginMethod(lastUsed, m.method))
		if (lastIdx > 0) all.unshift(...all.splice(lastIdx, 1))
		return all
	})

	let showPassword = $state(false)
	let passwordField = $state<Password | undefined>(undefined)
	// Type argument rather than annotation: annotating narrows the declaration to the
	// initializer's `undefined`, so a top-level read sees `never` instead of the array.
	let logins = $state<OAuthLogin[] | undefined>(undefined)
	let saml: string | undefined = $state(undefined)
	let smtpConfigured: boolean | undefined = $state(undefined)
	let disablePasswordLogin = $state(false)
	let autoRedirecting = $state(false)
	let oauthFlowDone = false

	// The method that worked last time leads: the form takes the top of the card, already open.
	let passwordFirst = $derived(lastUsedPassword && !disablePasswordLogin && !autoRedirecting)

	// Errors that belong to the credentials the user just submitted: they stay under the
	// password field until either field changes, so a stale message can't outlive its attempt.
	let formError = $state<
		| {
				message: string
				fields: 'both' | 'email' | 'password'
				email: string | undefined
				password: string | undefined
		  }
		| undefined
	>(undefined)
	let shake = $state(false)
	let fieldsEl = $state<HTMLDivElement | undefined>(undefined)
	let credentialsError = $derived(
		formError && formError.email === email && formError.password === password
			? formError.message
			: undefined
	)
	// 'both' sits under the password field, at the end of the form, where a rejected
	// credential pair belongs; a single missing field gets the message under itself.
	let errorField = $derived(credentialsError ? (formError?.fields ?? 'both') : undefined)
	let emailErrored = $derived(errorField === 'email' || errorField === 'both')
	let passwordErrored = $derived(errorField === 'password' || errorField === 'both')

	async function failLogin(
		message: string,
		fields: 'both' | 'email' | 'password' = 'both',
		// The pair the message is about. Defaults to what is in the fields right now, but a
		// rejected request passes what it actually submitted: the user may have typed on since.
		attempted: { email: string | undefined; password: string | undefined } = { email, password }
	) {
		// The shake is for a retry that fails the same way: on the first failure the message
		// appearing is the signal, and shaking it in would be noise.
		const wasAlreadyShown = credentialsError != undefined
		formError = { message, fields, ...attempted }
		// tick() only writes the DOM. Without a layout read between the removal and the re-add,
		// the browser coalesces both into one style recalculation, sees a class that never left,
		// and replays nothing from the second retry onwards.
		shake = false
		if (!wasAlreadyShown) return
		await tick()
		void fieldsEl?.offsetWidth
		shake = true
	}

	type OAuthLogin = {
		type: string
		displayName: string
	}

	async function login(): Promise<void> {
		if (!email || !password) {
			if (!email && !password) failLogin('Enter both your email and password.')
			else if (!email) failLogin('Enter your email.', 'email')
			else failLogin('Enter your password.', 'password')
			return
		}

		if (previewConfig) {
			failLogin('Invalid email or password.')
			return
		}

		const requestBody = {
			email,
			password
		}

		// Await the DOM update: the field must be back to type="password" before the
		// request goes out, or the browser may not offer to save the credential
		passwordField?.conceal()
		await tick()

		try {
			await UserService.login({ requestBody })
		} catch (err) {
			failLogin(loginErrorMessage(err), 'both', requestBody)
			return
		}

		formError = undefined
		rememberLoginMethod({ kind: 'password' })

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
		// Reduce same-origin full URLs to relative paths so deep links from
		// e.g. /a/[...path] (which carry the full URL as rd) still navigate
		// correctly instead of falling through to the cross-origin branch.
		let resolvedRd = toSameOriginRelativePath(rd) ?? rd
		if (resolvedRd?.startsWith('http')) {
			if (isValidLogoutRedirect(resolvedRd)) {
				window.location.href = resolvedRd
				return
			}
			goto('/')
			return
		}
		if ($workspaceStore) {
			goto(resolvedRd ?? '/')
		} else {
			let workspaceTarget = parseQueryParams(resolvedRd ?? undefined)['workspace']
			if (resolvedRd && workspaceTarget) {
				$workspaceStore = workspaceTarget
				goto(resolvedRd)
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
						const prefix = defaultApp.default_app_raw ? '/apps_raw/get' : '/apps/get'
						goto(`${prefix}/${defaultApp.default_app_path}`)
					} else {
						goto(resolvedRd ?? '/')
					}
				} else {
					goto(resolvedRd ?? '/')
				}
				// See (root)/+layout.svelte for why /projects/import skips the picker.
			} else if (
				resolvedRd?.startsWith('/user/workspaces') ||
				resolvedRd?.startsWith(`${base}/projects/import`)
			) {
				goto(resolvedRd)
			} else if (resolvedRd == '/#user-settings') {
				goto(`/user/workspaces#user-settings`)
			} else {
				goto(`/user/workspaces${resolvedRd ? `?rd=${encodeURIComponent(resolvedRd)}` : ''}`)
			}
		}
	}

	async function loadLogins() {
		if (previewConfig) {
			logins = previewConfig.logins ?? []
			saml = previewConfig.saml ? 'https://idp.example.com/sso' : undefined
			disablePasswordLogin = previewConfig.disablePasswordLogin ?? false
			autoRedirecting = previewConfig.autoRedirecting ?? false
			lastUsed = previewConfig.lastUsed
			showPassword =
				!disablePasswordLogin &&
				(lastUsedPassword ||
					(logins.length === 0 && !saml) ||
					(email != undefined && email.length > 0))
			onOptionsLoaded?.({ hasThirdParty: logins.length > 0 || !!saml })
			return
		}

		const [loginsResult, disabledResult] = await Promise.allSettled([
			OauthService.listOauthLogins(),
			UserService.isPasswordLoginDisabled()
		])

		if (disabledResult.status === 'fulfilled') {
			disablePasswordLogin = disabledResult.value ?? false
		} else {
			disablePasswordLogin = false
			console.error('Could not load password login setting', disabledResult.reason)
		}

		let autoLogin: string | undefined = undefined
		if (loginsResult.status === 'fulfilled') {
			logins = loginsResult.value.oauth.map((login) => ({
				type: login.type,
				displayName: login.display_name || login.type
			}))
			saml = loginsResult.value.saml
			autoLogin = loginsResult.value.auto_login
		} else {
			logins = []
			saml = undefined
			console.error('Could not load logins', loginsResult.reason)
		}

		lastUsed = getLastLoginMethod()
		showPassword =
			!disablePasswordLogin &&
			(lastUsedPassword ||
				(logins?.length === 0 && !saml) ||
				(email != undefined && email.length > 0))

		onOptionsLoaded?.({ hasThirdParty: (logins?.length ?? 0) > 0 || !!saml })

		if (autoRedirect && autoLogin && !error && !shouldSkipAutoRedirect()) {
			if (autoLogin === 'saml' && saml) {
				autoRedirecting = true
				if (!redirectSaml()) autoRedirecting = false
			} else if (logins?.some((l) => l.type === autoLogin)) {
				autoRedirecting = true
				if (!storeRedirect(autoLogin, true)) {
					autoRedirecting = false
					sendUserToast('Popup blocked — please click the sign-in button to continue.', true)
				}
			}
		}
	}

	function shouldSkipAutoRedirect(): boolean {
		try {
			const params = new URLSearchParams(window.location.search)
			return params.get('no_sso') === '1'
		} catch {
			return false
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
		if (previewConfig) {
			smtpConfigured = previewConfig.smtpConfigured ?? false
			return
		}
		try {
			smtpConfigured = await UserService.isSmtpConfigured()
		} catch (err) {
			console.error('Could not check if SMTP is configured', err)
			smtpConfigured = false
		}
	}

	checkSmtpConfigured()

	function handleKeyDown(event: KeyboardEvent) {
		const key = event.key

		// keydown auto-repeats while held, and Enter also confirms an IME candidate —
		// either would submit the form more than once per keypress
		if (key === 'Enter' && !event.isComposing && !event.repeat) {
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
			clearPendingLoginMethod()
			sendUserToast(data.error, true)
			onLoginError?.(data.error)
		} else if (data.type === 'success') {
			finishOauthFlow('postMessage')
		}
	}

	function handleStorageEvent(event) {
		if (event.key === 'oauth-success') {
			try {
				const data = JSON.parse(event.newValue)
				console.log('oauth-success from storage')
				// Clean up
				localStorage.removeItem('oauth-success')
				window.removeEventListener('storage', handleStorageEvent)
				if (data?.type === 'success') {
					finishOauthFlow('storage')
				} else {
					processPopupData(data)
				}
			} catch (e) {
				console.error('Could not process oauth-success from storage', e)
			}
		} else {
			console.log('Storage event', event.key)
		}
	}

	function finishOauthFlow(via: 'postMessage' | 'storage' | 'poll', win?: Window) {
		if (oauthFlowDone) return
		oauthFlowDone = true
		confirmPendingLoginMethod()
		console.log(`oauth: signaled via ${via}`)
		if (win && !win.closed) win.close()
		window.removeEventListener('message', popupListener)
		window.removeEventListener('storage', handleStorageEvent)
		onLoginSuccess?.()
	}

	onDestroy(() => {
		window.removeEventListener('message', popupListener)
		window.removeEventListener('storage', handleStorageEvent)
	})

	function persistRd() {
		if (!rd) return
		// Only persist a same-origin relative path. Storing a full URL pollutes
		// the localStorage key with a value that the post-login redirect logic
		// can't reuse safely (it would fall through the open-redirect guard).
		const safe = toSameOriginRelativePath(rd)
		if (!safe) return
		try {
			localStorage.setItem('rd', safe)
		} catch (e) {
			console.error('Could not persist redirection to local storage', e)
		}
	}

	// `automatic` marks the auto-login redirect, the one login that has to reach the
	// provider without drawing anything. It suppresses the provider's extra params —
	// Google's and Microsoft's account chooser — which every other login gets.
	function storeRedirect(provider: string, automatic: boolean): boolean {
		// The kitchen sink renders real provider buttons; clicking one must not leave the page.
		if (previewConfig) return true
		markLoginMethodPending({ kind: 'oauth', provider })
		persistRd()
		const params = new URLSearchParams()
		if (popup) params.set('close', 'true')
		if (automatic) params.set('auto', 'true')
		if (guestApp) params.set('guest_app', guestApp)
		const query = params.size > 0 ? '?' + params.toString() : ''
		let url = base + '/api/oauth/login/' + provider + query
		console.log('storeRedirect', popup, url)

		if (popup) {
			localStorage.setItem('closeUponLogin', 'true')
			window.addEventListener('message', popupListener)
			window.addEventListener('storage', handleStorageEvent)
			const win = window.open(url, '_blank', 'popup')
			if (!win) {
				window.removeEventListener('message', popupListener)
				window.removeEventListener('storage', handleStorageEvent)
				clearPendingLoginMethod()
				return false
			}
			// Safety net for Safari: when the popup is opened without a fresh user
			// gesture (auto-login), ITP can partition cookies/localStorage between
			// popup and parent, so neither the close cookie, the postMessage, nor
			// the localStorage 'oauth-success' signal reaches us. The session
			// cookie is set same-origin and isn't subject to that partitioning, so
			// polling whoami catches the success and lets us force-close the popup.
			pollForLoginSuccess(win)
			return true
		} else {
			localStorage.setItem('closeUponLogin', 'false')
			window.location.href = url
			return true
		}
	}

	function pollForLoginSuccess(win: Window) {
		const startedAt = Date.now()
		const interval = setInterval(async () => {
			if (oauthFlowDone) {
				clearInterval(interval)
				return
			}
			if (Date.now() - startedAt > 5 * 60 * 1000) {
				clearInterval(interval)
				console.log('oauth: poll timed out after 5 minutes')
				return
			}
			if (win.closed) {
				clearInterval(interval)
				console.log('oauth: popup closed before login completed')
				return
			}
			// A guest session is pinned to its workspace and cannot answer the global
			// probe; an ordinary session for a non-member cannot answer the workspace
			// one. Either answering means the popup signed someone in.
			const guestWorkspace = guestApp?.split('/')[0]
			const probes: Promise<unknown>[] = [UserService.getCurrentEmail()]
			if (guestWorkspace) probes.push(UserService.whoami({ workspace: guestWorkspace }))
			try {
				await Promise.any(probes)
			} catch {
				return
			}
			clearInterval(interval)
			finishOauthFlow('poll', win)
		}, 1500)
	}

	function redirectSaml(): boolean {
		if (!saml) {
			sendUserToast('No SAML login available', true)
			return false
		}
		if (previewConfig) return true
		markLoginMethodPending({ kind: 'saml' })
		let target = saml
		let relayStateSet = false
		// Carry the SP-initiated deep link through the IdP round-trip via SAML
		// RelayState so the ACS redirects straight back to it (bypassing
		// /user/login). Same-origin relative paths are passed verbatim;
		// full URLs (e.g. the page URL from /a/[...path]) are reduced to their
		// path component first. The backend re-validates. Cross-origin or
		// otherwise unsafe values fall through to the localStorage fallback.
		// A guest entry rides in the same RelayState as a `guest_app` query parameter
		// the ACS lifts out: SAML never passes through `/api/oauth/login/<client>`,
		// where the OAuth path hands its target to the server.
		let safePath = toSameOriginRelativePath(rd)
		if (guestApp && safePath) {
			const hashAt = safePath.indexOf('#')
			const pathAndQuery = hashAt === -1 ? safePath : safePath.slice(0, hashAt)
			const hash = hashAt === -1 ? '' : safePath.slice(hashAt)
			const sep = pathAndQuery.includes('?') ? '&' : '?'
			safePath = `${pathAndQuery}${sep}guest_app=${encodeURIComponent(guestApp)}${hash}`
		}
		if (safePath) {
			try {
				const url = new URL(saml)
				url.searchParams.set('RelayState', safePath)
				target = url.toString()
				relayStateSet = true
			} catch (e) {
				console.error('Could not set SAML RelayState', e)
			}
		}
		if (guestApp && !relayStateSet) {
			// Without the target the callback provisions an account, so a guest
			// sign-in that cannot carry it does not start.
			clearPendingLoginMethod()
			sendUserToast('Could not start sign-in, please try again.', true)
			return false
		}
		// Only use the localStorage fallback when RelayState is NOT carrying the
		// deep link. With RelayState the ACS redirects straight to the target and
		// /user/login never consumes/clears the key, so a persisted value would
		// go stale and hijack a later plain visit to /user/login.
		if (!relayStateSet) {
			persistRd()
		}
		window.location.href = target
		return true
	}

	$effect(() => {
		error && sendUserToast(escapeHtml(error), true)
	})
</script>

<!-- The red borders are colour-only, so role="alert" is what makes a failed attempt reach a
	screen reader. -->
{#snippet errorMessage()}
	<div id={errorId} role="alert" class="min-h-5">
		{#if errorField !== 'email'}
			<InputError error={credentialsError} />
		{/if}
	</div>
{/snippet}

<!-- Straddles the button's top edge, so the row keeps its height and the badge reads as a
	label on the button rather than another line of content. -->
{#snippet lastUsedBadge()}
	<!-- Hung off the corner like a notification badge: a long provider name wraps to two lines
		inside the button, and anything sitting further in would land on the label. The ring is
		the card's own colour so the badge punches through the button border. -->
	<div class="absolute top-0 right-0 -translate-y-1/2 translate-x-1/4 z-10">
		<Badge color="blue" small class="ring-2 ring-surface !px-1 !py-0 !text-[10px] leading-4">
			Last used
		</Badge>
	</div>
{/snippet}

{#snippet providerButtons()}
	<div class="grid gap-4 {autoRedirecting ? 'hidden' : ''}">
		{#if !logins}
			{#each Array(4) as _}
				<Skeleton layout={[0.5, [2.375]]} />
			{/each}
		{:else}
			{#each orderedThirdParty as entry (entry.method.kind === 'saml' ? 'saml:' : `oauth:${entry.method.provider}`)}
				<div class="relative">
					{#if sameLoginMethod(lastUsed, entry.method)}
						{@render lastUsedBadge()}
					{/if}
					<Button
						variant="default"
						unifiedSize="lg"
						startIcon={entry.icon ? { icon: entry.icon, classes: 'h-4' } : undefined}
						onClick={() =>
							entry.method.kind === 'saml'
								? redirectSaml()
								: storeRedirect(entry.method.provider, false)}
					>
						Continue with {entry.displayName}
					</Button>
				</div>
			{/each}
		{/if}
	</div>
{/snippet}

{#snippet orDivider()}
	<div class="flex items-center gap-3 my-6">
		<div class="h-px flex-1 bg-border-light"></div>
		<span class="text-2xs uppercase text-secondary">or</span>
		<div class="h-px flex-1 bg-border-light"></div>
	</div>
{/snippet}

<div class="bg-surface px-4 py-8 border sm:rounded-lg sm:px-10">
	{#if autoRedirecting}
		<p class="text-sm text-center text-secondary py-4">Signing you in…</p>
	{/if}

	{#if !passwordFirst}
		{@render providerButtons()}
		{#if !autoRedirecting && !disablePasswordLogin && (saml || (logins && logins.length > 0))}
			{@render orDivider()}
			<!-- Only an entry point to the form below: once that is open the divider is what
				separates the two ways in. -->
			{#if !showPassword}
				<div class="center-center">
					<Button
						unifiedSize="sm"
						variant="subtle"
						onClick={() => {
							showPassword = true
						}}
					>
						Log in without third-party
					</Button>
				</div>
			{/if}
		{/if}
	{/if}

	{#if !autoRedirecting && showPassword && !disablePasswordLogin}
		<div>
			{#if firstTime}
				<p class="text-xs text-center w-full pb-4 text-secondary">
					Welcome! Default credentials admin@windmill.dev / changeme have been prefilled.
				</p>
			{/if}
			<div class="space-y-4">
				{#if cloudHosted}
					<p class="text-xs text-secondary pb-6">
						To get credentials without the OAuth providers above, send an email at
						contact@windmill.dev
					</p>
				{/if}
				<div bind:this={fieldsEl} class="space-y-2 {shake ? 'motion-safe:animate-shake' : ''}">
					<div class="space-y-1">
						<label for={emailId} class="block text-xs font-semibold text-emphasis"> Email </label>
						<div>
							<TextInput
								size="md"
								error={emailErrored}
								bind:value={email}
								inputProps={{
									id: emailId,
									type: 'email',
									autocomplete: 'username',
									'aria-invalid': emailErrored ? 'true' : undefined,
									'aria-describedby':
										errorField === 'email' ? emailErrorId : emailErrored ? errorId : undefined,
									onkeydown: (e) => {
										// Only move on once the field holds something: while the browser's
										// credential dropdown is open, Enter belongs to the dropdown
										if (e.key === 'Enter' && !e.isComposing && !e.repeat && e.currentTarget.value) {
											e.preventDefault()
											passwordField?.focus()
										}
									}
								}}
							/>
						</div>
						<div id={emailErrorId} role="alert" class="min-h-5">
							{#if errorField === 'email'}
								<InputError error={credentialsError} />
							{/if}
						</div>
					</div>

					<div class="space-y-1">
						<label for={passwordId} class="block text-xs font-semibold text-emphasis">
							Password
						</label>
						<div>
							<Password
								bind:this={passwordField}
								bind:password
								id={passwordId}
								placeholder=""
								autocomplete="current-password"
								allowMultiline={false}
								error={passwordErrored}
								describedBy={passwordErrored ? errorId : undefined}
								onKeyDown={handleKeyDown}
							/>
						</div>
						{@render errorMessage()}
					</div>
				</div>

				<div>
					<Button onClick={login} variant="accent" unifiedSize="lg" disabled={!email || !password}>
						Log in
					</Button>
					{#if smtpConfigured}
						<div class="text-center pt-2">
							<a
								href="{base}/user/forgot-password"
								class="text-2xs text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
							>
								Forgot password?
							</a>
						</div>
					{/if}
				</div>
			</div>

			{#if cloudHosted}
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

	{#if passwordFirst && (saml || (logins && logins.length > 0))}
		{@render orDivider()}
		{@render providerButtons()}
	{/if}
</div>
