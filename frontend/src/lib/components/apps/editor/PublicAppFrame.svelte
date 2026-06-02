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
	import { Alert, Skeleton } from '$lib/components/common'
	import { base } from '$app/paths'
	import Login from '$lib/components/Login.svelte'

	type EmbedToken = { token?: string | null }

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

	// ---------------------------- embedder mode ----------------------------
	let status: 'loading' | 'ready' | 'noPermission' | 'notExists' = $state('loading')
	let embedToken: string | null = $state(null)
	let iframeEl: HTMLIFrameElement | undefined = $state(undefined)

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
			status = 'ready'
			// If the iframe already loaded (re-mint case), push the fresh token.
			postTokenToIframe()
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

	// Persistence authority (WIN-2006): the opaque viewer (and, nested under it,
	// any raw-app bundle) has no real Web Storage, so the embedder — which sits on
	// the real origin — persists their `localStorage` here, namespaced per app, and
	// serves reads synchronously. `wm_ls_*` = the viewer SPA's own storage;
	// `wm_raw_ls_*` = a raw bundle's storage relayed up by RawAppPreview.
	function viewerStoreKey(): string {
		return 'wm_appstore:' + page.url.pathname
	}
	function rawStoreKey(ns: string): string {
		return 'wm_rawstore:' + ns
	}
	function readStore(key: string): Record<string, string> {
		try {
			return JSON.parse(localStorage.getItem(key) || '{}')
		} catch (_) {
			return {}
		}
	}
	function writeStore(key: string, data: Record<string, string>) {
		try {
			localStorage.setItem(key, JSON.stringify(data || {}))
		} catch (_) {}
	}

	function handleEmbedderMessage(e: MessageEvent) {
		// The viewer is opaque-origin (e.origin === 'null'), so we authenticate
		// the message by source identity only.
		if (e.source !== iframeEl?.contentWindow) return
		if (e.data?.type === 'wm_embed_ready') {
			postTokenToIframe()
		} else if (e.data?.type === 'wm_embed_unauthorized') {
			initEmbedder()
		} else if (e.data?.type === 'wm_ls_req') {
			iframeEl?.contentWindow?.postMessage(
				{ type: 'wm_ls_set', data: readStore(viewerStoreKey()) },
				'*'
			)
		} else if (e.data?.type === 'wm_ls_sync') {
			writeStore(viewerStoreKey(), e.data.data)
		} else if (e.data?.type === 'wm_raw_ls_req') {
			iframeEl?.contentWindow?.postMessage(
				{ type: 'wm_raw_ls_set', ns: e.data.ns, data: readStore(rawStoreKey(e.data.ns)) },
				'*'
			)
		} else if (e.data?.type === 'wm_raw_ls_sync') {
			writeStore(rawStoreKey(e.data.ns), e.data.data)
		}
	}

	onMount(() => {
		if (isViewer) {
			window.addEventListener('message', handleViewerMessage)
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
