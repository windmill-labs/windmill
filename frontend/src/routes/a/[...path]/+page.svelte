<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

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

	function isJwt(t: string) {
		// A JWT is three dot-separated base64url segments; check the header decodes to
		// JSON. `atob` wants standard base64, so normalise base64url first (a `kid` or a
		// signature routinely contains `-`/`_`), or a valid token is taken for a path.
		try {
			const parts = t.split('.')
			if (parts.length !== 3) return false
			const b64 = parts[0].replace(/-/g, '+').replace(/_/g, '/')
			const pad = b64.length % 4 === 0 ? '' : '='.repeat(4 - (b64.length % 4))
			JSON.parse(atob(b64 + pad))
			return true
		} catch (e) {
			return false
		}
	}

	// The custom path may carry a trailing credential: an external JWT as its last
	// segment, or a guest JWT in a `guest.<jwt>` last segment (`<path>/guest.<jwt>`). The
	// `guest.` prefix keeps the two apart; `viewerUrl` uses `path` alone, so neither
	// reaches the opaque iframe.
	function parseCustomPath(customPath: string): {
		path: string
		jwt: string | undefined
		guestJwt: string | undefined
	} {
		const parts = customPath.split('/')
		const last = parts[parts.length - 1]
		// A guest JWT rides the last segment prefixed `guest.`. The `.` means it can never
		// be a valid custom-path segment, so a real path ending in a `guest` segment
		// followed by an external JWT (`.../guest/<jwt>`) is read as before, not hijacked.
		if (last.startsWith('guest.') && isJwt(last.slice('guest.'.length))) {
			return {
				path: parts.slice(0, -1).join('/'),
				jwt: undefined,
				guestJwt: last.slice('guest.'.length)
			}
		}
		if (parts.length > 1 && isJwt(last)) {
			return { path: parts.slice(0, -1).join('/'), jwt: last, guestJwt: undefined }
		}
		return { path: customPath, jwt: undefined, guestJwt: undefined }
	}

	const parsedCustomPath = parseCustomPath(page.params.path ?? '')

	// URL for the opaque viewer iframe: the custom-path URL WITHOUT the trailing
	// JWT segment. The JWT is a viewer credential (broader and longer-lived than
	// the scoped embed token) consumed here on the embedder side only — it must
	// never appear in the iframe's own location, where app-authored code could
	// read it. Captured once (not reactively): the embedder mirrors the app's
	// hash/query back onto this page's URL, and re-deriving the src from it would
	// reload the app on its every navigation.
	const viewerUrl = `${base}/a/${parsedCustomPath.path}${page.url.search}${page.url.hash}`

	let workspace: string | undefined = $state(undefined)
	let refresh: (() => void) | undefined
	/** `<workspace>/<app_path>` when this app is open to guests. Resolved eagerly:
	 * PublicAppFrame renders its sign-in gate before `onViewerReady` fires. */
	let guestAppPath: string | undefined = $state(undefined)
	/** The frame's sign-in card must not mount before this is known: a configured
	 * auto-login would otherwise fire an ordinary sign-in and provision an account.
	 * Only the card waits — the app load itself runs in parallel with discovery.
	 * Only a confirmed 404 means "not a guest app"; any other failure is `error`, since
	 * offering an ordinary sign-in on a transient fault would provision an account. */
	let guestEntry: 'pending' | 'none' | 'guest' | 'error' = $state('pending')

	// Settled once: `loadApp` calls this again on failure, and a later transient fault
	// must not overwrite an answer already in hand. A function, not a narrowed local:
	// the value changes across the awaits below.
	const guestEntrySettled = () => guestEntry === 'guest' || guestEntry === 'none'
	async function loadGuestEntry() {
		if (guestEntrySettled()) return
		for (let attempt = 0; attempt < 3; attempt++) {
			try {
				const entry = await AppService.getGuestEntryByCustomPath({
					customPath: parsedCustomPath.path
				})
				guestAppPath = `${entry.workspace_id}/${entry.app_path}`
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

	// Embedder side: validate access (main session cookie or shared JWT) and mint
	// a scoped embed token for the opaque iframe (WIN-2006).
	async function fetchEmbedToken(opts?: { sdkConsent?: boolean }): Promise<{ token?: string }> {
		if (parsedCustomPath.guestJwt) {
			OpenAPI.TOKEN = 'jwt_guest_' + parsedCustomPath.guestJwt
		} else if (parsedCustomPath.jwt) {
			OpenAPI.TOKEN = 'jwt_ext_' + parsedCustomPath.jwt
		}
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const consent = opts?.sdkConsent ? '?sdk_consent=true' : ''
		const res = await fetch(
			`${OpenAPI.BASE}/apps_u/embed_token_by_custom_path/${parsedCustomPath.path}${consent}`,
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
			app = await AppService.getPublicAppByCustomPath({
				customPath: parsedCustomPath.path
			})
			workspace = app.workspace_id
			workspaceStore.set(app.workspace_id)
			noPermission = false
			notExists = false
			jwtError = false

			try {
				userStore.set(await getUserExt(app.workspace_id))
				// A JWT in the custom path that fails to resolve a user is surfaced as a
				// toast (matches the pre-sandbox custom-path viewer) rather than silently
				// falling through to anonymous.
				if (!$userStore && parsedCustomPath.jwt) {
					jwtError = true
					sendUserToast('Could not authentify user with jwt token', true)
				}
			} catch (e) {
				console.warn('Anonymous user')
			}
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

	if (BROWSER) {
		setLicense()
		loadGuestEntry()
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
			{workspace}
			{notExists}
			{noPermission}
			{jwtError}
			{guestAppPath}
			{app}
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>
