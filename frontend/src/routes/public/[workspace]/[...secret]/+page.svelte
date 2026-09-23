<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore } from '$lib/stores'

	import { setLicense } from '$lib/enterpriseUtils'

	import { getUserExt } from '$lib/user'
	import { page } from '$app/state'
	import { base } from '$lib/base'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'

	let app: (AppWithLastVersion & { value: any }) | undefined = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	let jwtError = $state(false)
	/** `<workspace>/<app_path>` when this app is open to guests, so the sign-in card can
	 * offer a guest session rather than a dead end. 404 (the common case) leaves it
	 * undefined. */
	let guestAppPath: string | undefined = $state(undefined)
	/** The frame's sign-in card must not mount before this is known: a configured
	 * auto-login would otherwise fire an ordinary sign-in and provision an account.
	 * Only the card waits — the app load itself runs in parallel with discovery.
	 * Only a confirmed 404 means "not a guest app"; any other failure is `error`, since
	 * offering an ordinary sign-in on a transient fault would provision an account. */
	let guestEntry: 'pending' | 'none' | 'guest' | 'error' = $state('pending')

	// The share link carries a trailing credential the embedder consumes: an external
	// JWT as `<secret>/<jwt>`, or a guest JWT as `<secret>/guest.<jwt>`. The `guest.`
	// prefix keeps the two apart with no parsing of the token, which the page cannot
	// verify anyway. Either way `viewerUrl` below uses `secret` alone, so no JWT
	// reaches the opaque iframe.
	function parseSecret(secret: string): {
		secret: string
		jwt: string | undefined
		guestJwt: string | undefined
	} {
		const parts = secret.split('/')
		// The credential rides the segment after the secret: a guest JWT prefixed
		// `guest.`, or an external JWT bare. The `guest.` prefix glues the marker to the
		// token, so it can never be mistaken for a path or secret segment (which carry no
		// `.`), and a bare token keeps the established external-JWT interpretation.
		if (parts[1]?.startsWith('guest.')) {
			return { secret: parts[0], jwt: undefined, guestJwt: parts[1].slice('guest.'.length) }
		}
		return { secret: parts[0], jwt: parts[1], guestJwt: undefined }
	}

	const parsedSecret = parseSecret(page.params.secret ?? '')
	const workspace = page.params.workspace ?? ''

	// URL for the opaque viewer iframe: the share URL WITHOUT the trailing JWT
	// segment. The JWT is a viewer credential (broader and longer-lived than the
	// scoped embed token) consumed here on the embedder side only — it must never
	// appear in the iframe's own location, where app-authored code could read it.
	// Captured once (not reactively): the embedder mirrors the app's hash/query
	// back onto this page's URL, and re-deriving the src from it would reload the
	// app on its every navigation.
	const viewerUrl = `${base}/public/${workspace}/${parsedSecret.secret}${page.url.search}${page.url.hash}`

	let refresh: (() => void) | undefined

	// Embedder side: validate access (using the main session cookie or the shared
	// JWT) and mint a scoped embed token for the opaque iframe (WIN-2006).
	async function fetchEmbedToken(opts?: { sdkConsent?: boolean }): Promise<{ token?: string }> {
		if (parsedSecret.guestJwt) {
			OpenAPI.TOKEN = 'jwt_guest_' + parsedSecret.guestJwt
		} else if (parsedSecret.jwt) {
			OpenAPI.TOKEN = 'jwt_ext_' + parsedSecret.jwt
		}
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const consent = opts?.sdkConsent ? '?sdk_consent=true' : ''
		const res = await fetch(
			`${OpenAPI.BASE}/w/${workspace}/apps_u/embed_token/${parsedSecret.secret}${consent}`,
			{ headers }
		)
		if (!res.ok) {
			const err: any = new Error('Failed to fetch embed token')
			err.status = res.status
			throw err
		}
		return await res.json()
	}

	// Viewer side: load the app + user using the embed token handed to the iframe.
	async function loadApp() {
		try {
			userStore.set(await getUserExt(workspace))
		} catch (e) {
			console.warn('Anonymous user')
		}
		try {
			app = await AppService.getPublicAppBySecret({
				workspace,
				path: parsedSecret.secret
			})
			noPermission = false
			notExists = false
		} catch (e) {
			if (e.status == 401) {
				// Embed token missing/expired — ask the embedder for a fresh one.
				refresh?.()
			} else {
				notExists = true
			}
			// The app exists and admits guests; the load failed only for want of a
			// session, so offer one instead of the not-found page.
			await loadGuestEntry()
			if (guestAppPath) {
				notExists = false
				noPermission = true
			}
		}
	}

	// Settled once: `loadApp` calls this again on failure, and a later transient fault
	// must not overwrite an answer already in hand. A function, not a narrowed local:
	// the value changes across the awaits below.
	const guestEntrySettled = () => guestEntry === 'guest' || guestEntry === 'none'
	async function loadGuestEntry() {
		if (guestEntrySettled()) return
		for (let attempt = 0; attempt < 3; attempt++) {
			try {
				const entry = await AppService.getGuestEntry({ workspace, path: parsedSecret.secret })
				guestAppPath = `${workspace}/${entry.app_path}`
				guestEntry = 'guest'
				return
			} catch (e) {
				if (e?.status === 404) {
					guestAppPath = undefined
					guestEntry = 'none'
					return
				}
				await new Promise((r) => setTimeout(r, 500 * (attempt + 1)))
				// A concurrent call may have settled it meanwhile.
				if (guestEntrySettled()) return
			}
		}
		if (!guestEntrySettled()) {
			guestAppPath = undefined
			guestEntry = 'error'
		}
	}

	// Eager, not on the failure path: PublicAppFrame asks for the embed token and
	// renders its own sign-in gate before `onViewerReady` ever fires, so resolving
	// this only after a failed `loadApp` would be too late for the case that matters
	// most — a signed-out visitor.
	if (BROWSER) {
		loadGuestEntry()
	}

	if (BROWSER) {
		setLicense()
	}
</script>

<PublicAppFrame
	{fetchEmbedToken}
	{viewerUrl}
	{guestAppPath}
	{guestEntry}
	onViewerReady={(_token, requestTokenRefresh) => {
		refresh = requestTokenRefresh
		loadApp()
	}}
>
	{#snippet viewer()}
		<PublicApp
			{app}
			{workspace}
			{notExists}
			{noPermission}
			{jwtError}
			{guestAppPath}
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>
