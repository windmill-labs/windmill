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
	import type { UserExt } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { Pen } from 'lucide-svelte'
	import { page } from '$app/state'
	import {
		setOperatingWorkspace,
		useOperatingUser
	} from '$lib/components/operatingWorkspace.svelte'

	let {
		workspace,
		path,
		onEdit,
		onLoadState
	}: {
		workspace: string
		path: string
		/** Handle Edit in place instead of following `editHref`. An AI session shows
		 * this viewer inside a preview tab, where a plain link would navigate the whole
		 * page out of the session rather than flipping the tab to its editor. */
		onEdit?: () => void
		/** How the load ended, for a host that renders its own state around this viewer.
		 * A 403 is deliberately neither: the app exists, this member just cannot open it. */
		onLoadState?: (state: 'loaded' | 'not_found') => void
	} = $props()

	// The app, its permission check and everything this viewer renders belong to `workspace`,
	// which on a session preview tab is the session's and not the one the nav points at.
	setOperatingWorkspace(() => workspace)
	const operatingUser = useOperatingUser()

	/** This workspace's membership for the viewer, which is what the app's `ctx.username` /
	 * `ctx.groups` must describe — they sit beside `ctx.workspace` in the same object. */
	let appUser: UserExt | undefined = $state(undefined)
	let app: any = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	/** The app's own permissions, kept apart from the verdict: `loadPerms` resolves before the
	 * acting user's `whoami` does in a workspace that is not the navigation one, and a verdict
	 * computed there would stick at "cannot write" with nothing to recompute it. */
	let appPerms = $state<{ path: string; extraPerms: Record<string, boolean> } | undefined>(
		undefined
	)
	const canWriteApp = $derived(
		!!appPerms && canWrite(appPerms.path, appPerms.extraPerms, operatingUser.current)
	)
	/** Raw vs low-code, read from the app itself rather than from the route:
	 * both kinds render here and either route serves either kind (links to a raw
	 * app point at /apps/get all over the app), so only the app can say which
	 * editor the Edit button must open. */
	let isRawApp = $state(false)
	let editHref = $derived(`${base}/${isRawApp ? 'apps_raw' : 'apps'}/edit/${path}?nodraft=true`)
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
	async function fetchEmbedToken(opts?: { sdkConsent?: boolean }): Promise<{ token?: string }> {
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const consent = opts?.sdkConsent ? '?sdk_consent=true' : ''
		const res = await fetch(`${OpenAPI.BASE}/w/${workspace}/apps/embed_token/p/${path}${consent}`, {
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
		// Kept local and handed to PublicApp rather than written to `userStore`: this is the
		// membership in `workspace`, and a session preview tab shows a workspace the rest of
		// the page is not on — writing it globally would answer every permission check on
		// that page (the sidebar, the session bar's fork button) for the wrong workspace.
		// The routes that mount this are all under `(logged)`, whose layout has already
		// populated `userStore` for the workspace the page *is* on.
		appUser = await getUserExt(workspace)
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
			onLoadState?.('loaded')
		} catch (e: any) {
			if (e.status == 401) refresh?.()
			else if (e.status == 403) noPermission = true
			else {
				notExists = true
				onLoadState?.('not_found')
			}
		}
	}

	// Edit button: determine write access and which editor to open on this
	// real-origin page (cookie). The sandboxed low-code app never loads on this
	// page (it loads inside the opaque iframe), so `app` can't be the source.
	async function loadPerms() {
		try {
			const lite: any = await AppService.getAppLiteByPath({ workspace, path })
			appPerms = { path: lite?.path ?? path, extraPerms: lite?.extra_perms ?? {} }
			isRawApp = !!lite?.raw_app
		} catch (_) {
			appPerms = undefined
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
			user={appUser}
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
		<Button
			size="sm"
			startIcon={{ icon: Pen }}
			variant="subtle"
			href={onEdit ? undefined : editHref}
			on:click={() => onEdit?.()}>Edit</Button
		>
	</div>
{/if}
