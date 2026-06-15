<script lang="ts">
	/*
	 * WIN-2006: shared in-workspace app viewer (low-code AND raw). Renders the app
	 * through the same PublicAppFrame -> PublicApp machinery the public viewer uses,
	 * so the sandbox behavior is identical on every page. PublicAppFrame picks the
	 * rendering: an opaque /app_embed iframe for sandboxed low-code, or inline (with
	 * the bundle isolated in RawAppPreview's own opaque iframe) for raw and for
	 * unsandboxed apps. When the publisher opts an app into sandbox isolation, its
	 * untrusted markup/JS must not run with the member's full session — hence the
	 * scoped embed token / opaque isolation.
	 */
	import { base } from '$lib/base'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'
	import { Button } from '$lib/components/common'
	import { AppService, OpenAPI } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { Pen } from 'lucide-svelte'
	import { page } from '$app/state'

	let {
		workspace,
		path,
		editHref
	}: {
		workspace: string
		path: string
		/** Where the Edit button points (low-code vs raw editor). */
		editHref: string
	} = $props()

	let app: any = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	let canWriteApp = $state(false)
	let refresh: (() => void) | undefined

	// The opaque iframe loads the dedicated cookieless, chrome-less viewer route.
	// The page's query/hash are forwarded so the app sees the same `ctx.query` /
	// `ctx.hash` as the pre-sandbox viewer did. Captured ONCE (not reactively):
	// the embedder later mirrors the app's own hash/query changes back onto this
	// page's URL (wm_embed_hash relay), and re-deriving the iframe src from them
	// would reload the app on its every navigation.
	const initialSearchHash = page.url.search + page.url.hash
	let viewerUrl = $derived(`${base}/app_embed/${workspace}/${path}${initialSearchHash}`)

	const hideEditBtn = page.url.searchParams.get('hideEditBtn') === 'true'
	const hideRefreshBar = page.url.searchParams.get('hideRefreshBar') === 'true'

	// Embedder side: mint a scoped embed token (by path) from the member's session.
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

	// Viewer side — used for the inline renderings (raw, and unsandboxed low-code);
	// the sandboxed low-code case loads inside the opaque /app_embed iframe instead.
	// getAppByPath returns bundle_secret + runnables for raw apps, which
	// PublicApp -> RawAppPreview needs.
	async function loadApp() {
		try {
			userStore.set(await getUserExt(workspace))
		} catch (e) {
			console.warn('Anonymous user')
		}
		try {
			const loaded: any = await AppService.getAppByPath({ workspace, path })
			// Raw apps need the bundle secret to load their bundle. getAppByPath
			// doesn't compute it (unlike the public handlers), so fetch it here — the
			// same call the previous raw viewer used — and hand it to PublicApp ->
			// RawAppPreview via bundle_secret.
			if (loaded?.raw_app && !loaded.bundle_secret) {
				try {
					loaded.bundle_secret = await AppService.getPublicSecretOfLatestVersionOfApp({
						workspace,
						path
					})
				} catch (e) {
					console.error('Failed to load raw app bundle secret', e)
				}
			}
			app = loaded
			noPermission = false
			notExists = false
		} catch (e: any) {
			if (e.status == 401) refresh?.()
			else if (e.status == 403) noPermission = true
			else notExists = true
		}
	}

	// Edit button: determine write access on this real-origin page (cookie).
	async function loadPerms() {
		try {
			const lite: any = await AppService.getAppLiteByPath({ workspace, path })
			canWriteApp = canWrite(lite?.path, lite?.extra_perms ?? {}, $userStore)
		} catch (_) {
			canWriteApp = false
		}
	}

	$effect(() => {
		if (workspace && path) loadPerms()
	})
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
			jwtError={false}
			inWorkspace
			{hideRefreshBar}
			onLoginSuccess={() => loadApp()}
		></PublicApp>
	{/snippet}
</PublicAppFrame>

{#if canWriteApp && !hideEditBtn}
	<div id="app-edit-btn" class="absolute bottom-4 z-50 right-4">
		<Button size="sm" startIcon={{ icon: Pen }} variant="subtle" href={editHref}>Edit</Button>
	</div>
{/if}
