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

	function parseSecret(secret: string): { secret: string; jwt: string | undefined } {
		const parts = secret.split('/')
		return {
			secret: parts[0],
			jwt: parts[1]
		}
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
	async function fetchEmbedToken(): Promise<{ token?: string }> {
		if (parsedSecret.jwt) {
			OpenAPI.TOKEN = 'jwt_ext_' + parsedSecret.jwt
		}
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const res = await fetch(
			`${OpenAPI.BASE}/w/${workspace}/apps_u/embed_token/${parsedSecret.secret}`,
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
		}
	}

	if (BROWSER) {
		setLicense()
	}
</script>

<PublicAppFrame
	{fetchEmbedToken}
	{viewerUrl}
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
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>
