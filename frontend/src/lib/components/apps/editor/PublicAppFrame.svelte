<script lang="ts">
	/*
	 * WIN-2006: published apps and raw apps render arbitrary user-authored
	 * markup/JS. That code is untrusted (a malicious author, or an XSS bug in the
	 * app itself) and must not run with the viewer's session. So the app is always
	 * rendered inside an **opaque-origin** (sandboxed, no `allow-same-origin`)
	 * iframe and handed a **narrowly-scoped embed token** — never the session
	 * cookie.
	 *
	 * This component plays two roles, selected by the `wm_embed=1` query param +
	 * whether the window is framed:
	 *
	 *  - Embedder (top-level window): authenticates the viewer (using the main
	 *    session cookie or a shared JWT), mints a scoped embed token, renders an
	 *    opaque iframe pointing at the same route, and hands it the token via
	 *    postMessage. The cookie never crosses to the iframe.
	 *  - Viewer (framed, opaque origin): receives the token and uses it as the
	 *    *only* credential for API calls, then renders the actual app.
	 *
	 * Unlike a separate-domain setup, no `PUBLIC_APP_DOMAIN` is required: the
	 * opaque origin is, for same-origin-policy purposes, as foreign to the main
	 * app as a different domain would be. The iframe's API calls are therefore
	 * cross-origin and rely on `Access-Control-Allow-Origin: *` + the bearer
	 * token (no cookie), and the served document carries `CSP: sandbox` so direct
	 * navigation to it is opaque too.
	 */
	import { BROWSER } from 'esm-env'
	import { OpenAPI } from '$lib/gen'
	import { page } from '$app/state'
	import { onDestroy, onMount, setContext, type Snippet } from 'svelte'
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import { base } from '$app/paths'
	import Login from '$lib/components/Login.svelte'
	import { TriangleAlert } from 'lucide-svelte'

	type EmbedToken = { token?: string | null; disable_sandbox?: boolean; version?: number | null }

	let {
		fetchEmbedToken,
		onViewerReady,
		viewer
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
	} = $props()

	const EMBED_PARAM = 'wm_embed'
	const ORIGIN_PARAM = 'wm_embedder_origin'

	const framed = BROWSER && window.parent !== window
	const isViewer = BROWSER && page.url.searchParams.get(EMBED_PARAM) === '1' && framed

	// ----------------------------- viewer mode -----------------------------
	let viewerToken: string | undefined = $state(undefined)
	let viewerReady = $state(false)
	const expectedEmbedderOrigin = BROWSER ? page.url.searchParams.get(ORIGIN_PARAM) : null

	// Components that embed the token in a URL (images, PDFs, downloads) read it
	// from the `AuthToken` context. In viewer mode that must be the embed token;
	// the getter keeps it in sync once the token arrives.
	setContext<{ token?: string }>('AuthToken', {
		get token() {
			return viewerToken
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
		viewerReady = false
		window.parent.postMessage({ type: 'wm_embed_unauthorized' }, expectedEmbedderOrigin ?? '*')
	}

	// WIN-2006: the app runs inside the (opaque) viewer iframe, so its in-app URL
	// changes never reach the top address bar — breaking shareable deep links. Relay
	// the hash up to the embedder, which mirrors it onto its own URL. Hash only: it
	// can't spoof the path (the embedder keeps its own pathname), so a hostile app
	// can't rewrite the address bar to an unrelated route.
	let origPushState: typeof history.pushState | undefined
	let origReplaceState: typeof history.replaceState | undefined
	function relayHash() {
		try {
			window.parent.postMessage(
				{ type: 'wm_embed_hash', hash: window.location.hash },
				expectedEmbedderOrigin ?? '*'
			)
		} catch (_) {}
	}
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

	// Read by RawAppPreview (and any app component) to render same-origin (full
	// access) instead of the sandboxed bundle iframe.
	setContext('IS_APP_UNSANDBOXED', {
		get value() {
			return disableSandbox && consented
		}
	})

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
		const url = new URL(window.location.href)
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
			appVersion = resp.version ?? undefined
			status = 'ready'
			if (disableSandbox) {
				// Same-origin (full-session) mode behind a per-version consent prompt.
				consented = hasConsent()
				if (consented) onViewerReady?.(undefined, requestTokenRefresh)
			} else {
				// If the iframe already loaded (re-mint case), push the fresh token.
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
			// Mirror the viewer's in-app hash onto our own URL (shareable deep links).
			// Keep our pathname/search; only adopt the hash.
			const hash = typeof e.data.hash === 'string' ? e.data.hash : ''
			if (window.location.hash !== hash) {
				history.replaceState(null, '', window.location.pathname + window.location.search + hash)
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
{:else if disableSandbox}
	<!-- Publisher disabled sandbox isolation: run same-origin (full session) only
	     after explicit, per-version viewer consent. -->
	{#if consented}
		{@render viewer()}
	{:else}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
			<div class="bg-surface max-w-lg w-full rounded-lg shadow-lg p-6 space-y-4">
				<div class="flex items-center gap-2 text-orange-600">
					<TriangleAlert size={22} />
					<h2 class="text-lg font-semibold">Run this app without isolation?</h2>
				</div>
				<p class="text-sm text-secondary">
					The publisher of <span class="font-mono">{page.url.pathname}</span> has disabled sandbox isolation.
					This app will run with access to your Windmill session and can act on your behalf. Only continue
					if you trust the publisher.
				</p>
				<p class="text-xs text-tertiary">You'll be asked again when the app is updated.</p>
				<div class="flex justify-end gap-2">
					<Button variant="default" color="light" href={base}>Cancel</Button>
					<Button variant="contained" color="red" onclick={acceptConsent}>Run app</Button>
				</div>
			</div>
		</div>
	{/if}
{:else}
	<iframe
		bind:this={iframeEl}
		src={buildViewerUrl()}
		title="App"
		class="w-full h-screen border-0 block"
		sandbox="allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals allow-top-navigation"
		allow="clipboard-read; clipboard-write; fullscreen"
	></iframe>
{/if}
