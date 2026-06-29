<script lang="ts">
	/*
	 * WIN-2006: in-workspace app viewer, sandboxed. This is the private analog of
	 * the public `/public/[workspace]/[...secret]` route — same `PublicAppFrame`
	 * embedder/viewer + scoped-token machinery, but the app is addressed by path and
	 * the token is minted from the logged-in member's session (cookie) via the
	 * authenticated `apps/embed_token/p/{path}` endpoint. It lives outside `(logged)`
	 * so the opaque (cookieless) viewer iframe can load it without the auth redirect
	 * or the workspace chrome. `/apps/get` embeds this route in an opaque iframe.
	 */
	import { BROWSER } from 'esm-env'
	import { AppService, OpenAPI } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { setLicense } from '$lib/enterpriseUtils'
	import { getUserExt } from '$lib/user'
	import { page } from '$app/state'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'

	let app: any = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)

	const workspace = page.params.workspace ?? ''
	const path = page.params.path ?? ''
	// Forwarded by InWorkspaceAppViewer from the member-facing page's query.
	const hideRefreshBar = page.url.searchParams.get('hideRefreshBar') === 'true'
	let refresh: (() => void) | undefined

	// Embedder side: the logged-in member's session mints a scoped embed token for
	// the opaque iframe, isolating the in-workspace app from their full session.
	async function fetchEmbedToken(): Promise<{ token?: string }> {
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const res = await fetch(`${OpenAPI.BASE}/w/${workspace}/apps/embed_token/p/${path}`, {
			headers
		})
		if (!res.ok) {
			const err: any = new Error('Failed to fetch embed token')
			err.status = res.status
			throw err
		}
		return await res.json()
	}

	// Viewer side: load the app + user using the embed token handed to the iframe.
	async function loadApp() {
		// Set the workspace store so app job operations (notably JobLoader.cancelJob,
		// which reads $workspaceStore) target this workspace instead of an empty/stale
		// one in the cookieless iframe.
		workspaceStore.set(workspace)
		try {
			userStore.set(await getUserExt(workspace))
		} catch (e) {
			console.warn('Anonymous user')
		}
		try {
			app = await AppService.getAppByPath({ workspace, path })
			noPermission = false
			notExists = false
		} catch (e: any) {
			if (e.status == 401) {
				// Embed token missing/expired — ask the embedder for a fresh one.
				refresh?.()
			} else if (e.status == 403) {
				noPermission = true
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
			{app}
			{workspace}
			{notExists}
			{noPermission}
			jwtError={false}
			inWorkspace
			{hideRefreshBar}
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>
