<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'

	import { setLicense } from '$lib/enterpriseUtils'

	import { getUserExt } from '$lib/user'
	import { page } from '$app/state'
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

	let workspace: string | undefined = $state(undefined)
	let refresh: (() => void) | undefined

	// Embedder side: validate access (main session cookie or shared JWT) and mint
	// a scoped embed token for the opaque iframe (WIN-2006).
	async function fetchEmbedToken(): Promise<{ token?: string }> {
		if (parsedCustomPath.jwt) {
			OpenAPI.TOKEN = 'jwt_ext_' + parsedCustomPath.jwt
		}
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const res = await fetch(
			`${OpenAPI.BASE}/apps_u/embed_token_by_custom_path/${parsedCustomPath.path}`,
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

			try {
				userStore.set(await getUserExt(app.workspace_id))
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
		}
	}

	if (BROWSER) {
		setLicense()
	}
</script>

<PublicAppFrame
	{fetchEmbedToken}
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
			{app}
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>
