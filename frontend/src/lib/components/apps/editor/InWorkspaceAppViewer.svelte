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
	import { sendUserToast } from '$lib/toast'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { pageHeader, PHONE_BAR } from '$lib/components/pageHeaderRegistry.svelte'
	import type { Item } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import { AppService, OpenAPI } from '$lib/gen'
	import { userStore, type UserExt } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { ExternalLink, Pen } from 'lucide-svelte'
	import { page } from '$app/state'
	import { isMenuHidden } from '$lib/components/sessions/sessionMode.svelte'
	import {
		setOperatingWorkspace,
		useOperatingUser
	} from '$lib/components/operatingWorkspace.svelte'

	let {
		workspace,
		path,
		onEdit,
		onLoadState,
		syncHashToUrl = true,
		ownsPageHeader = false
	}: {
		workspace: string
		path: string
		/** True for the route's own page: the app becomes what the page header names. A host that
		 * renders this inside something else — a session's preview panel, whose band names the
		 * session — leaves it false, and the app keeps its Edit over its own canvas. */
		ownsPageHeader?: boolean
		/** Handle Edit in place instead of following `editHref`. An AI session shows
		 * this viewer inside a preview tab, where a plain link would navigate the whole
		 * page out of the session rather than flipping the tab to its editor. */
		onEdit?: () => void
		/** How the load ended, for a host that renders its own state around this viewer.
		 * A 403 is deliberately neither: the app exists, this member just cannot open it. */
		onLoadState?: (state: 'loaded' | 'not_found') => void
		/** Whether a raw app's route lives in the page URL's hash; see `RawAppPreview`. */
		syncHashToUrl?: boolean
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
	// `canWrite` reads folder and ownership rights only, and apps.rs refuses an operator's writes
	// whatever those say.
	const canWriteApp = $derived(
		!!appPerms &&
			!operatingUser.current?.operator &&
			canWrite(appPerms.path, appPerms.extraPerms, operatingUser.current)
	)
	/** Raw vs low-code, read from the app itself rather than from the route:
	 * both kinds render here and either route serves either kind (links to a raw
	 * app point at /apps/get all over the app), so only the app can say which
	 * editor the Edit button must open. */
	let isRawApp = $state(false)
	/** The app's execution mode, from the same lite read as the permissions and for the same
	 * reason: a sandboxed low-code app loads inside the opaque iframe, so `app` here stays
	 * undefined and cannot say whether the app answers at a public url. `policy.execution_mode` is
	 * where the lite payload carries it; the flat field beside it is the apps *list* row's
	 * spelling (ListableApp, which is what AppRow reads). */
	let executionMode = $state<string | undefined>(undefined)
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

	const showEdit = $derived(canWriteApp && !hideEditBtn)
	// Decides where Edit goes, not whether it appears: without the workspace navigation there is
	// no band to put it in. Same rule the layout hides the sidebar by.
	const menuHidden = $derived(isMenuHidden(page.url))

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
				// Only a 404 is "nothing deployed here"; anything else is a failure to say so.
				if (e.status == 404) onLoadState?.('not_found')
				else sendUserToast('Could not load app: ' + (e.body ?? e.message ?? e), true)
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
			executionMode = lite?.policy?.execution_mode ?? lite?.execution_mode
		} catch (_) {
			appPerms = undefined
		}
	}

	/** Whether the app answers at a public url at all. Read from the app this viewer has already
	 *  loaded, the way the apps table decides the same thing (AppRow) — the secret itself is a
	 *  request, and asking for one on every app view to decide whether to draw a button is a
	 *  request per view that most views never use. */
	const hasPublicUrl = $derived(executionMode === 'anonymous')
	/** Resolved when the button is used, not when it is drawn. */
	async function openPublicUrl() {
		// The tab is opened inside the click itself and filled once the secret arrives. Safari only
		// lets a gesture open the tab it opens synchronously, so opening it after the await is
		// blocked there — silently, since a blocked popup reports nothing.
		const tab = window.open('about:blank', '_blank')
		try {
			const secret = await AppService.getPublicSecretOfApp({ workspace, path })
			if (!secret) {
				tab?.close()
				sendUserToast('This app has no public url', true)
				return
			}
			// Built from this viewer's workspace rather than the navigation one, like everything
			// else here: the two differ inside a session's preview panel.
			const url = `${window.location.origin}${base}/public/${workspace}/${secret}`
			if (tab) tab.location.href = url
			else sendUserToast('Allow pop-ups for this site to open the app’s public url', true)
		} catch (e: any) {
			tab?.close()
			sendUserToast('Could not open the public url: ' + (e?.body ?? e?.message ?? e), true)
		}
	}

	$effect(() => {
		if (workspace && path) loadPerms()
	})

	// Both of this page's buttons carry a label, and a phone's bar has room for neither beside the
	// app's path — so below that width they become one menu. Unmeasured (0) counts as wide: the bar
	// measures itself on mount, and starting compact would pop them out a frame later.
	const compact = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < PHONE_BAR)
	const compactItems: Item[] = $derived([
		...(hasPublicUrl
			? [
					{
						displayName: 'Public url',
						icon: ExternalLink,
						action: () => openPublicUrl()
					}
				]
			: []),
		{
			displayName: 'Edit',
			icon: Pen,
			href: onEdit ? undefined : editHref,
			action: onEdit ? () => onEdit() : undefined
		}
	])
</script>

<div class="h-full">
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
				user={appUser ?? $userStore}
				{notExists}
				{noPermission}
				jwtError={false}
				inWorkspace
				{hideRefreshBar}
				{syncHashToUrl}
				onLoginSuccess={() => loadApp()}
			></PublicApp>
		{/snippet}
	</PublicAppFrame>
</div>

<!-- The band names the app for whoever opened it, write access or not: the route alone registers
     no item, and the breadcrumb would fall back to the section name "Apps". -->
{#if ownsPageHeader}
	<PageHeaderContent
		item={{ kind: 'app', path }}
		actions={showEdit && !menuHidden ? editAction : undefined}
		separator="always"
	/>
{/if}

<!-- Edit is the bar's action when this viewer owns the bar, and floats over the canvas otherwise:
     an embed has no bar — it would cost the app 44px of the iframe to carry one button — and
     inside a session's preview panel the bar belongs to the session, so an Edit up there would
     sit beside the session's name and act on the panel below it. -->
{#if showEdit && (menuHidden || !ownsPageHeader)}
	<div class="absolute bottom-4 right-4 z-50">
		{@render editAction()}
	</div>
{/if}

{#snippet editAction()}
	{#if compact}
		<!-- Too little bar to seat both: they fold into one menu rather than losing their labels,
		     which are what say where each one goes. -->
		<DropdownV2 items={compactItems} placement="bottom-end" size="sm" />
	{:else}
		{#if hasPublicUrl}
			<!-- The app on its own, at the url anyone it is shared with uses. A new tab rather than
			     this one: the viewer here is the same app, so replacing it would look like nothing
			     happened. The secret is fetched by the click, so the url exists only once asked for. -->
			<Button
				unifiedSize="sm"
				variant="subtle"
				startIcon={{ icon: ExternalLink }}
				onclick={openPublicUrl}
				title="Open the app's public url in a new tab">Public url</Button
			>
		{/if}
		<!-- `onEdit` wins over the href: a host that embeds this viewer opens its own editor rather
		     than navigating the frame to one. -->
		<Button
			unifiedSize="sm"
			startIcon={{ icon: Pen }}
			variant="default"
			href={onEdit ? undefined : editHref}
			on:click={() => onEdit?.()}
			title="Edit this app"
			id="app-edit-btn">Edit</Button
		>
	{/if}
{/snippet}
