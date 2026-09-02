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
		// simply check that the first part is a valid base64 encoded json
		try {
			const parts = t.split('.')
			const header = atob(parts[0])
			JSON.parse(header)
			return true
		} catch (e) {
			return false
		}
	}

	function parseCustomPath(customPath: string): { path: string; jwt: string | undefined } {
		const parts = customPath.split('/')
		if (parts.length > 1 && isJwt(parts[parts.length - 1])) {
			return {
				path: parts.slice(0, -1).join('/'),
				jwt: parts[parts.length - 1]
			}
		} else {
			return {
				path: customPath,
				jwt: undefined
			}
		}
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

	async function loadGuestEntry() {
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
			}
		}
		guestAppPath = undefined
		guestEntry = 'error'
	}

	// Embedder side: validate access (main session cookie or shared JWT) and mint
	// a scoped embed token for the opaque iframe (WIN-2006).
	async function fetchEmbedToken(opts?: { sdkConsent?: boolean }): Promise<{ token?: string }> {
		if (parsedCustomPath.jwt) {
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
