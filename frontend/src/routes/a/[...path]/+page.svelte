<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'

	import { setLicense } from '$lib/enterpriseUtils'

	import { getUserExt } from '$lib/user'
	import { page } from '$app/state'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'

	let app: (AppWithLastVersion & { value: any; workspace_id?: string }) | undefined =
		$state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	let jwtError = $state(false)
	let workspace: string | undefined = $state(undefined)

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

	let refresh: (() => void) | undefined

	// Embedder side: validate access + mint a scoped token for the iframe.
	async function fetchEmbedToken() {
		if (parsedCustomPath.jwt) {
			OpenAPI.TOKEN = 'jwt_ext_' + parsedCustomPath.jwt
		}
		return await AppService.getAppEmbedTokenByCustomPath({
			customPath: parsedCustomPath.path
		})
	}

	// Viewer side: load the app + user using the embed token handed to the iframe.
	async function loadApp() {
		try {
			app = await AppService.getPublicAppByCustomPath({
				customPath: parsedCustomPath.path
			})
			workspace = app.workspace_id
			if (app.workspace_id) {
				workspaceStore.set(app.workspace_id)
			}
			noPermission = false
			notExists = false

			try {
				if (app.workspace_id) {
					userStore.set(await getUserExt(app.workspace_id))
				}
			} catch (e) {
				console.warn('Anonymous user')
			}
		} catch (e) {
			if (e.status == 401) {
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
