<script lang="ts">
	/*
	 * WIN-2006: published apps render arbitrary user-authored markup/JS. That code
	 * is untrusted (a malicious author, or an XSS bug in the app) and must not run
	 * with the viewer's session. How it's contained depends on the app:
	 *
	 *  - Low-code (default): rendered in an **opaque-origin** (sandboxed, no
	 *    `allow-same-origin`) iframe and handed a **narrowly-scoped embed token** —
	 *    never the session cookie. This component is the embedder (top window:
	 *    authenticates the viewer, mints the token, renders the opaque iframe) and,
	 *    inside that iframe (`wm_embed=1`), the viewer (uses the token as its only
	 *    credential, then renders the app).
	 *  - Raw (default): rendered directly here as a **single** opaque bundle iframe
	 *    (the author bundle is already isolated in its own opaque iframe); no opaque
	 *    viewer / embed token needed.
	 *  - Unsandboxed (same-origin): grandfathered "legacy" apps, an anonymous viewer
	 *    of a publisher `disable_sandbox` app, or an authenticated viewer who has
	 *    consented (per app version). Rendered directly with full session access.
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
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { base } from '$app/paths'
	import { goto } from '$app/navigation'
	import Login from '$lib/components/Login.svelte'
	import { WINDMILL_RESERVED_QUERY_PARAMS } from '$lib/utils'
	import { EMBED_NAV_CONTEXT_KEY, type EmbedNav } from '../types'

	type EmbedToken = {
		token?: string | null
		disable_sandbox?: boolean
		version?: number | null
		raw_app?: boolean
		legacy_unsandboxed?: boolean
		authed?: boolean
	}

	let {
		fetchEmbedToken,
		onViewerReady,
		viewer,
		viewerUrl,
		appPath
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
		/** App path, shown in the consent prompt when known (the public viewer may
		 * not know it before the app loads — the prompt then says "this app"). */
		appPath?: string
	} = $props()

	const EMBED_PARAM = 'wm_embed'
	const ORIGIN_PARAM = 'wm_embedder_origin'

	const framed = BROWSER && window.parent !== window
	const isViewer = BROWSER && page.url.searchParams.get(EMBED_PARAM) === '1' && framed

	// ----------------------------- viewer mode -----------------------------
	let viewerToken: string | undefined = $state(undefined)
	let viewerReady = $state(false)
	const expectedEmbedderOrigin = BROWSER ? page.url.searchParams.get(ORIGIN_PARAM) : null

	// Components that embed the token in a URL (images, PDFs, downloads, SSE) read
	// it from the `AuthToken` context. In viewer mode that must be the embed token;
	// the getter keeps it in sync once the token arrives. In direct render (legacy /
	// consented / raw — the viewer snippet runs on this page, not in the opaque
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
			onViewerReady?.(token, requestTokenRefresh)
		}
	}

	/** Passed to the viewer: when the rendered app gets a 401 (e.g. the embed
	 * token expired) it calls this to ask the embedder for a fresh token. */
	function requestTokenRefresh() {
		if (!isViewer) {
			// Embedder-side direct render (raw app, or consented disable-sandbox): there
			// is no opaque viewer to message — just re-run the access check / app load.
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

	// WIN-2006: publisher disabled sandbox isolation. The app then runs same-origin
	// with the viewer's full session (full browser features), gated by a consent
	// prompt that must be accepted once per app version.
	let disableSandbox = $state(false)
	let appVersion: number | undefined = $state(undefined)
	let consented = $state(false)
	// WIN-2006: authenticated FOR THIS WORKSPACE — drives the embed-token mint, not
	// the consent gate (that uses `hasSession`).
	let authed = $state(false)
	// WIN-2006: whether this browser holds any Windmill session (the domain-wide
	// cookie), even for another workspace. The disable_sandbox consent gate keys off
	// THIS, not `authed`: a same-origin app receives the cookie, so a viewer logged
	// into a different workspace must consent before it runs with their session.
	let hasSession = $state(false)
	// WIN-2006: grandfathered app (existed before sandboxing shipped) — runs
	// same-origin with NO consent prompt, to avoid breaking the installed base on
	// upgrade. Distinct from disable_sandbox, which is a deliberate publisher opt-out.
	let legacyUnsandboxed = $state(false)

	// WIN-2006 Variant A: raw apps render single-iframe. The author's bundle is
	// already isolated in its own opaque iframe (served + CSP-sandboxed by the
	// backend), so the viewer skips the opaque-viewer indirection and the embed
	// token entirely — it renders the app directly on this (real) origin and the
	// bridge calls the backend with the page credential (cookie / JWT / anonymous),
	// matching the logged-in raw viewer. Low-code keeps the opaque viewer + token.
	let isRaw = $state(false)

	// Effective same-origin (unsandboxed) execution: legacy apps always; a
	// publisher disable_sandbox app when the viewer has no Windmill session (nothing
	// to expose) or a session-holding viewer has consented.
	let unsandboxed = $derived(legacyUnsandboxed || (disableSandbox && (!hasSession || consented)))
	// Consent is required only for a viewer who holds a Windmill session looking at
	// a non-legacy disable_sandbox app that hasn't consented yet.
	let needsConsent = $derived(disableSandbox && hasSession && !legacyUnsandboxed && !consented)

	// Read by RawAppPreview (and any app component) to render same-origin (full
	// access) instead of the sandboxed bundle iframe.
	setContext('IS_APP_UNSANDBOXED', {
		get value() {
			return unsandboxed
		}
	})

	// WIN-2006: does this browser hold a Windmill session for ANY workspace?
	// Cookie-only (no Authorization header, so the embed JWT isn't sent — we want
	// the viewer's real login, not the app identity). On a separate
	// PUBLIC_APP_DOMAIN the cookie is absent, so this returns false.
	async function browserHasWindmillSession(): Promise<boolean> {
		try {
			const resp = await fetch(`${OpenAPI.BASE}/users/whoami`, {
				credentials: 'include',
				headers: { accept: 'application/json' }
			})
			return resp.ok
		} catch (_) {
			return false
		}
	}

	function consentKey(): string {
		return `wm_unsandboxed_ok:${page.url.pathname}:${appVersion ?? ''}`
	}
	function hasConsent(): boolean {
		try {
			return localStorage.getItem(consentKey()) === '1'
		} catch (_) {
			return false
		}
	}
	function acceptConsent() {
		try {
			localStorage.setItem(consentKey(), '1')
		} catch (_) {}
		consented = true
		onViewerReady?.(undefined, requestTokenRefresh)
	}

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
			disableSandbox = resp.disable_sandbox ?? false
			legacyUnsandboxed = resp.legacy_unsandboxed ?? false
			authed = resp.authed ?? false
			appVersion = resp.version ?? undefined
			isRaw = resp.raw_app ?? false
			// `authed` already implies a session; otherwise probe the cookie (only
			// needed for the consent case). MUST resolve before `status = 'ready'` so
			// the app never renders same-origin before we know whether to gate.
			hasSession = authed
			if (disableSandbox && !legacyUnsandboxed && !authed) {
				hasSession = await browserHasWindmillSession()
			}
			// Only a session-holding viewer of a non-legacy disable_sandbox app must
			// consent (once per version) before it runs with their session.
			if (disableSandbox && hasSession && !legacyUnsandboxed) {
				consented = hasConsent()
			}
			status = 'ready'

			const needConsentNow = disableSandbox && hasSession && !legacyUnsandboxed && !consented
			const unsandboxedNow = legacyUnsandboxed || (disableSandbox && (!hasSession || consented))

			if (needConsentNow) {
				// Wait for the viewer to accept (consent modal → acceptConsent).
			} else if (unsandboxedNow || isRaw) {
				// Render the app directly on this origin: same-origin when unsandboxed
				// (legacy / consented / anonymous opt-out), or a single opaque bundle
				// iframe when it's a sandboxed raw app.
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

	// Persistence authority (WIN-2006): opaque app frames (the viewer SPA and any
	// raw-app bundle nested under it) have no real Web Storage, so the embedder —
	// on the real origin — backs their `localStorage` here. It is a single store
	// **shared across all apps** (like one PUBLIC_APP_DOMAIN origin), kept under
	// one key and updated with per-key ops so concurrent frames merge instead of
	// clobbering.
	const SHARED_LS_KEY = 'wm_apps_localstorage'
	function readSharedLs(): Record<string, string> {
		try {
			return JSON.parse(localStorage.getItem(SHARED_LS_KEY) || '{}')
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
			localStorage.setItem(SHARED_LS_KEY, JSON.stringify(s))
		} catch (_) {}
	}

	function handleEmbedderMessage(e: MessageEvent) {
		// The viewer is opaque-origin (e.origin === 'null'), so we authenticate
		// the message by source identity only. Storage messages arrive either from
		// the viewer SPA's shim or relayed up by RawAppPreview (raw bundle) — both
		// target the same shared store.
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
{:else if needsConsent}
	<!-- Publisher disabled sandbox isolation and the viewer is authenticated: run
	     same-origin (full session) only after explicit, per-version consent.
	     Anonymous viewers and grandfathered (legacy) apps skip this. -->
	<ConfirmationModal
		open={needsConsent}
		title="Run this app without isolation?"
		confirmationText="Run app"
		onConfirmed={acceptConsent}
		onCanceled={() => {
			window.location.href = base || '/'
		}}
	>
		<p>
			The publisher of {#if appPath}<span class="font-mono">{appPath}</span>{:else}this app{/if} has
			disabled sandbox isolation. It will run with access to your Windmill session and can act on your
			behalf. Only continue if you trust the publisher.
		</p>
		<p class="mt-2 text-xs text-tertiary">You'll be asked again when the app is updated.</p>
	</ConfirmationModal>
{:else if unsandboxed}
	<!-- Same-origin (full session): a grandfathered legacy app, an anonymous viewer
	     of a disable_sandbox app, or an authenticated viewer who consented. Rendered
	     directly here; RawAppPreview reads IS_APP_UNSANDBOXED to drop the bundle's
	     opaque sandbox. -->
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
