<script lang="ts">
	/*
	 * WIN-2006: published apps render arbitrary user-authored markup/JS. When the
	 * publisher opts an app into sandbox isolation (alpha), that untrusted code (a
	 * malicious author, or an XSS bug in the app) must not run with the viewer's
	 * session. How it's contained depends on the app:
	 *
	 *  - Low-code, sandboxed: rendered in an **opaque-origin** (sandboxed, no
	 *    `allow-same-origin`) iframe and handed a **narrowly-scoped embed token** —
	 *    never the session cookie. This component is the embedder (top window:
	 *    authenticates the viewer, mints the token, renders the opaque iframe) and,
	 *    inside that iframe (`wm_embed=1`), the viewer (uses the token as its only
	 *    credential, then renders the app).
	 *  - Raw, sandboxed: rendered directly here as a **single** opaque bundle iframe
	 *    (the author bundle is already isolated in its own opaque iframe); no opaque
	 *    viewer / embed token needed.
	 *  - Unsandboxed (default): the app runs same-origin with the viewer's full
	 *    session, the pre-isolation behavior. Rendered directly here.
	 *
	 * No `PUBLIC_APP_DOMAIN` is required: the opaque origin is, for same-origin-policy
	 * purposes, as foreign to the main app as a different domain. The opaque viewer's
	 * API calls are cross-origin and rely on `Access-Control-Allow-Origin: *` + the
	 * bearer token (no cookie); the raw wrapper document always carries `CSP: sandbox`.
	 */
	import { BROWSER } from 'esm-env'
	import { OpenAPI } from '$lib/gen'
	import { page } from '$app/state'
	import { onDestroy, onMount, setContext, type Snippet } from 'svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { base } from '$app/paths'
	import { goto } from '$app/navigation'
	import Login from '$lib/components/Login.svelte'
	import { WINDMILL_RESERVED_QUERY_PARAMS } from '$lib/utils'
	import { EMBED_NAV_CONTEXT_KEY, IS_APP_ISOLATED_CONTEXT_KEY, type EmbedNav } from '../types'

	type EmbedToken = {
		token?: string | null
		raw_app?: boolean
		sandbox?: boolean
		app_path?: string | null
		workspace_id?: string | null
	}

	let {
		fetchEmbedToken,
		onViewerReady,
		viewer,
		viewerUrl
	}: {
		/** Embedder-side: validate access + mint the scoped token. Throws with a
		 * `.status` of 401 (login required) or 404 (not found). */
		fetchEmbedToken: () => Promise<EmbedToken>
		/** Viewer-side: fired (once per received token) when the embed token is
		 * available, before the app renders. Use it to kick off data loading.
		 * `requestTokenRefresh` asks the embedder for a fresh token on a 401. */
		onViewerReady?: (token: string | undefined, requestTokenRefresh: () => void) => void
		/** Viewer-side: renders the actual app once the embed token is available. */
		viewer: Snippet
		/** Embedder-side: override the opaque iframe src (the route that renders the
		 * viewer). Defaults to the current route + `wm_embed=1` (public routes embed
		 * themselves). The in-workspace viewer sets this because its embedder route
		 * (`/apps/get`, auth-gated, with chrome) differs from the cookieless,
		 * chrome-less viewer route (`/app_embed`). */
		viewerUrl?: string
	} = $props()

	const EMBED_PARAM = 'wm_embed'
	const ORIGIN_PARAM = 'wm_embedder_origin'

	const framed = BROWSER && window.parent !== window
	const isViewer = BROWSER && page.url.searchParams.get(EMBED_PARAM) === '1' && framed

	// Components that render author-authored markup read this to know they are on an
	// opaque origin. `unsandboxed` is NOT usable for that: it stays true in viewer mode
	// (initEmbedder only runs on the embedder), so it reads "unsandboxed" inside the
	// very iframe that is the sandbox.
	setContext(IS_APP_ISOLATED_CONTEXT_KEY, isViewer)

	// ----------------------------- viewer mode -----------------------------
	let viewerToken: string | undefined = $state(undefined)
	let viewerReady = $state(false)
	// Set when viewer mode never receives a token (see the orphan timer in onMount):
	// the page carries `wm_embed` but isn't actually framed by a Windmill embedder.
	let viewerOrphaned = $state(false)
	const expectedEmbedderOrigin = BROWSER ? page.url.searchParams.get(ORIGIN_PARAM) : null

	// Components that embed the token in a URL (images, PDFs, downloads, SSE) read
	// it from the `AuthToken` context. In viewer mode that must be the embed token;
	// the getter keeps it in sync once the token arrives. In direct render
	// (unsandboxed / raw — the viewer snippet runs on this page, not in the opaque
	// iframe) expose the page's own bearer credential when there is one: JWT public
	// URLs put it in `OpenAPI.TOKEN` (set before the app loads) and have no cookie
	// to fall back on. Cookie sessions have no bearer here and keep using the cookie.
	setContext<{ token?: string }>('AuthToken', {
		get token() {
			if (isViewer) return viewerToken
			return typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN ? OpenAPI.TOKEN : undefined
		}
	})

	function handleViewerMessage(e: MessageEvent) {
		// The embedder has a real origin, so we can validate both the source and
		// the origin of messages we receive.
		if (e.source !== window.parent) return
		if (expectedEmbedderOrigin && e.origin !== expectedEmbedderOrigin) return
		if (e.data?.type === 'wm_embed_token') {
			const token = e.data.token ?? undefined
			viewerToken = token
			// The bearer token (when present) is the credential the app uses; no
			// cookie reaches this opaque origin.
			OpenAPI.TOKEN = token
			viewerReady = true
			viewerOrphaned = false
			onViewerReady?.(token, requestTokenRefresh)
		}
	}

	/** Passed to the viewer: when the rendered app gets a 401 (e.g. the embed
	 * token expired) it calls this to ask the embedder for a fresh token. */
	function requestTokenRefresh() {
		if (!isViewer) {
			// Embedder-side direct render (raw app, or unsandboxed app): there is no
			// opaque viewer to message — just re-run the access check / app load.
			initEmbedder()
			return
		}
		viewerReady = false
		window.parent.postMessage({ type: 'wm_embed_unauthorized' }, expectedEmbedderOrigin ?? '*')
	}

	// WIN-2006: the app runs inside the (opaque) viewer iframe, so its in-app URL
	// changes never reach the top address bar — breaking shareable deep links. Relay
	// the hash + query up to the embedder, which mirrors them onto its own URL
	// (e.g. the navbar component's same-app links set ?query and #hash). Never the
	// path: the embedder keeps its own pathname, so a hostile app can't rewrite the
	// address bar to an unrelated route. Transport params (wm_embed, …) are
	// stripped before relaying.
	let origPushState: typeof history.pushState | undefined
	let origReplaceState: typeof history.replaceState | undefined
	function relayHash() {
		try {
			const params = new URLSearchParams(window.location.search)
			const reserved: string[] = []
			params.forEach((_v, k) => {
				if (WINDMILL_RESERVED_QUERY_PARAMS.has(k)) reserved.push(k)
			})
			reserved.forEach((k) => params.delete(k))
			const qs = params.toString()
			const search = qs ? `?${qs}` : ''
			window.parent.postMessage(
				{ type: 'wm_embed_hash', hash: window.location.hash, search },
				expectedEmbedderOrigin ?? '*'
			)
		} catch (_) {}
	}

	// Top-level navigation relay (WIN-2006): in-app links to other apps (navbar
	// "app" items) must navigate the top page — navigating inside the opaque
	// iframe would load the SPA cookieless. Consumed by PublicApp via gotoFn.
	setContext<EmbedNav | undefined>(
		EMBED_NAV_CONTEXT_KEY,
		isViewer
			? {
					navigateTop: (href: string) => {
						try {
							window.parent.postMessage(
								{ type: 'wm_embed_navigate', href },
								expectedEmbedderOrigin ?? '*'
							)
						} catch (_) {}
					}
				}
			: undefined
	)
	function installHashRelay() {
		origPushState = history.pushState
		origReplaceState = history.replaceState
		history.pushState = function (data, unused, url) {
			origPushState?.call(history, data, unused, url ?? null)
			relayHash()
		}
		history.replaceState = function (data, unused, url) {
			origReplaceState?.call(history, data, unused, url ?? null)
			relayHash()
		}
		window.addEventListener('hashchange', relayHash)
		window.addEventListener('popstate', relayHash)
	}
	function uninstallHashRelay() {
		if (origPushState) history.pushState = origPushState
		if (origReplaceState) history.replaceState = origReplaceState
		window.removeEventListener('hashchange', relayHash)
		window.removeEventListener('popstate', relayHash)
	}

	// ---------------------------- embedder mode ----------------------------
	let status: 'loading' | 'ready' | 'noPermission' | 'notExists' = $state('loading')
	let embedToken: string | null = $state(null)
	let iframeEl: HTMLIFrameElement | undefined = $state(undefined)

	// WIN-2006: publisher opted this app into sandbox isolation (alpha). When false
	// (the default) the app runs same-origin with the viewer's full session — the
	// pre-isolation behavior.
	let sandboxed = $state(false)

	// WIN-2006 Variant A: raw apps render single-iframe. The author's bundle is
	// already isolated in its own opaque iframe (served + CSP-sandboxed by the
	// backend), so the viewer skips the opaque-viewer indirection and the embed
	// token entirely — it renders the app directly on this (real) origin and the
	// bridge calls the backend with the page credential (cookie / JWT / anonymous),
	// matching the logged-in raw viewer. Low-code keeps the opaque viewer + token.
	let isRaw = $state(false)

	// WIN-2006: the resolved app path + workspace, used together to scope this app's
	// backing localStorage (below) so sandboxed apps don't share one store (even two
	// apps at the same path in different workspaces).
	let appPath: string | undefined = $state(undefined)
	let workspaceId: string | undefined = $state(undefined)

	// Same-origin (full session) execution: every app that wasn't opted into
	// sandbox isolation. RawAppPreview reads this to drop the bundle's opaque
	// sandbox; an unsandboxed low-code app renders directly here.
	let unsandboxed = $derived(!sandboxed)

	// Read by RawAppPreview (and any app component) to render same-origin (full
	// access) instead of the sandboxed bundle iframe.
	setContext('IS_APP_UNSANDBOXED', {
		get value() {
			return unsandboxed
		}
	})

	function buildViewerUrl(): string {
		// Default: embed the current route. The in-workspace viewer overrides this
		// with a dedicated cookieless, chrome-less viewer route (`/app_embed`).
		const url = new URL(viewerUrl ?? window.location.href, window.location.origin)
		url.searchParams.set(EMBED_PARAM, '1')
		url.searchParams.set(ORIGIN_PARAM, window.location.origin)
		// Same origin (no separate domain); the sandbox makes it opaque.
		return url.pathname + url.search + url.hash
	}

	async function initEmbedder() {
		status = 'loading'
		try {
			const resp = await fetchEmbedToken()
			embedToken = resp.token ?? null
			sandboxed = resp.sandbox ?? false
			isRaw = resp.raw_app ?? false
			appPath = resp.app_path ?? undefined
			workspaceId = resp.workspace_id ?? undefined
			status = 'ready'

			if (unsandboxed || isRaw) {
				// Render the app directly on this origin: same-origin when unsandboxed
				// (the default), or a single opaque bundle iframe when it's a sandboxed
				// raw app.
				onViewerReady?.(undefined, requestTokenRefresh)
			} else {
				// Sandboxed low-code: hand the scoped token to the opaque viewer iframe.
				postTokenToIframe()
			}
		} catch (e: any) {
			status = e?.status === 401 ? 'noPermission' : 'notExists'
		}
	}

	function postTokenToIframe() {
		// The iframe is an opaque origin ("null"), which cannot be named as a
		// targetOrigin, so we use '*'. This only relaxes the receiver-origin
		// check; the message is still delivered solely to our own iframe's
		// contentWindow, whose content we control.
		iframeEl?.contentWindow?.postMessage({ type: 'wm_embed_token', token: embedToken }, '*')
	}

	// Persistence authority (WIN-2006): opaque app frames (the viewer SPA) have no
	// real Web Storage, so the embedder — on the real origin — backs their
	// `localStorage` here. The store is scoped PER APP (keyed by workspace + app path)
	// so one sandboxed app cannot read or clobber another's storage. Updated with
	// per-key ops so concurrent frames of the same app merge instead of clobbering.
	function lsKey(): string {
		return `wm_apps_localstorage:${workspaceId ?? ''}:${appPath ?? ''}`
	}
	function readSharedLs(): Record<string, string> {
		try {
			return JSON.parse(localStorage.getItem(lsKey()) || '{}')
		} catch (_) {
			return {}
		}
	}
	function applyLsOp(d: any) {
		const s = readSharedLs()
		if (d.op === 'set') s[d.key] = String(d.value)
		else if (d.op === 'remove') delete s[d.key]
		else if (d.op === 'clear') for (const k in s) delete s[k]
		try {
			localStorage.setItem(lsKey(), JSON.stringify(s))
		} catch (_) {}
	}

	function handleEmbedderMessage(e: MessageEvent) {
		// The viewer is opaque-origin (e.origin === 'null'), so we authenticate
		// the message by source identity only. Storage messages come from the opaque
		// low-code viewer SPA's shim and are backed in this app's per-app store.
		if (e.source !== iframeEl?.contentWindow) return
		if (e.data?.type === 'wm_embed_ready') {
			postTokenToIframe()
		} else if (e.data?.type === 'wm_embed_unauthorized') {
			initEmbedder()
		} else if (e.data?.type === 'wm_ls_req') {
			iframeEl?.contentWindow?.postMessage({ type: 'wm_ls_hydrate', data: readSharedLs() }, '*')
		} else if (e.data?.type === 'wm_ls_op') {
			applyLsOp(e.data)
		} else if (e.data?.type === 'wm_embed_hash') {
			// Mirror the viewer's in-app hash + query onto our own URL (shareable
			// deep links). Keep our pathname (an app can't rewrite the address bar
			// to another route) and our own transport params (e.g. wm_coep).
			const hash = typeof e.data.hash === 'string' ? e.data.hash : ''
			const relayed = new URLSearchParams(
				typeof e.data.search === 'string' ? e.data.search : window.location.search
			)
			for (const k of WINDMILL_RESERVED_QUERY_PARAMS) {
				relayed.delete(k)
				const own = new URLSearchParams(window.location.search).get(k)
				if (own !== null) relayed.set(k, own)
			}
			const qs = relayed.toString()
			const search = qs ? `?${qs}` : ''
			if (window.location.hash !== hash || window.location.search !== search) {
				history.replaceState(null, '', window.location.pathname + search + hash)
			}
		} else if (e.data?.type === 'wm_embed_navigate') {
			// App-initiated same-window navigation (navbar app items, frontend-script
			// `goto`, button `gotoUrl`): this page navigates itself, exactly like when
			// the app ran on it pre-sandbox. Same-origin paths use SPA navigation;
			// full http(s) URLs do a real load (pre-sandbox apps could already do
			// this via window.location, so it grants nothing new). Everything else —
			// javascript:, data:, protocol-relative `//host` — is rejected.
			const href = typeof e.data.href === 'string' ? e.data.href : ''
			if (href.startsWith('/') && !href.startsWith('//')) {
				goto(href)
			} else if (/^https?:\/\//i.test(href)) {
				window.location.href = href
			}
		}
	}

	onMount(() => {
		if (isViewer) {
			window.addEventListener('message', handleViewerMessage)
			installHashRelay()
			// Announce readiness so the embedder sends us the token.
			window.parent.postMessage({ type: 'wm_embed_ready' }, expectedEmbedderOrigin ?? '*')
			// This page is only ever loaded as the embedder's opaque iframe, which
			// replies with the token within milliseconds. If none arrives, it was
			// opened with `wm_embed` outside a Windmill embedder (e.g. its iframe src
			// embedded directly in a third-party page); show a diagnostic instead of an
			// indefinite skeleton.
			const orphanTimer = setTimeout(() => {
				if (!viewerReady) viewerOrphaned = true
			}, 3000)
			return () => clearTimeout(orphanTimer)
		} else {
			window.addEventListener('message', handleEmbedderMessage)
			initEmbedder()
		}
	})

	onDestroy(() => {
		if (!BROWSER) return
		window.removeEventListener('message', handleViewerMessage)
		window.removeEventListener('message', handleEmbedderMessage)
		if (isViewer) uninstallHashRelay()
	})
</script>

{#if isViewer}
	{#if viewerReady}
		{@render viewer()}
	{:else if viewerOrphaned}
		<div class="px-4 mt-20 max-w-xl mx-auto">
			<Alert type="info" title="Open this app from Windmill">
				This is a Windmill app viewer and must be loaded by Windmill. If you embedded it in your own
				page, use the app's public URL without the <code>wm_embed</code> parameter.
			</Alert>
		</div>
	{:else}
		<Skeleton layout={[[4], 0.5, [50]]} />
	{/if}
{:else if status === 'loading'}
	<Skeleton layout={[[4], 0.5, [50]]} />
{:else if status === 'notExists'}
	<div class="px-4 mt-20">
		<Alert type="error" title="Not found">
			There was an error loading the app, is the url correct?
			<a href={base}>Go to Windmill</a>
		</Alert>
	</div>
{:else if status === 'noPermission'}
	<!-- Login happens here, on the embedder (main) window, so the session cookie
	     is set on the main origin only and never reaches the opaque iframe. -->
	<div class="px-4 mt-20 w-full text-center font-bold text-xl">This app requires read access</div>
	<div class="px-2 mx-auto mt-20 max-w-xl w-full">
		<Login
			onLoginSuccess={() => initEmbedder()}
			popup
			rd={page.url.pathname + page.url.search + page.url.hash}
		/>
	</div>
{:else if unsandboxed}
	<!-- Same-origin (full session): the app was not opted into sandbox isolation
	     (the default). Rendered directly here; RawAppPreview reads
	     IS_APP_UNSANDBOXED to drop the bundle's opaque sandbox. -->
	{@render viewer()}
{:else if isRaw}
	<!-- Variant A: sandboxed raw app rendered directly on the real origin. The
	     untrusted author bundle stays isolated in its own opaque iframe (inside
	     RawAppPreview); no opaque viewer and no embed token are needed. -->
	{@render viewer()}
{:else}
	<!-- referrerpolicy: the embedder page URL can carry a viewer credential (the
	     JWT path segment of share links); without this, the same-origin iframe
	     navigation would expose it to app-authored code via document.referrer. -->
	<iframe
		bind:this={iframeEl}
		src={buildViewerUrl()}
		title="App"
		class="w-full h-screen border-0 block"
		sandbox="allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals allow-top-navigation"
		allow="clipboard-read; clipboard-write; fullscreen"
		referrerpolicy="no-referrer"
	></iframe>
{/if}
