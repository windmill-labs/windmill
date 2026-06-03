<script lang="ts">
	/*
	 * WIN-2006: apps and raw apps are always rendered inside an iframe so that the
	 * (potentially XSS-compromised) app document is isolated from the main
	 * Windmill session.
	 *
	 * This component plays two roles depending on whether it is the top-level
	 * window (the "embedder") or the framed window (the "viewer"), distinguished
	 * by the `wm_embed=1` query param + being framed:
	 *
	 *  - Embedder: authenticates the viewer (using the main-domain session cookie
	 *    or a JWT), mints a narrowly-scoped embed token, then renders an iframe
	 *    pointing at the same URL (on `public_app_domain` if configured, else the
	 *    same origin) and hands it the token via postMessage. The session cookie
	 *    never crosses to the iframe origin.
	 *  - Viewer: receives the token from the embedder, uses it as the *only*
	 *    credential for API calls, and renders the actual app.
	 *
	 * When `public_app_domain` is unset the iframe is same-origin: the scoped
	 * token still constrains the app's own calls, but a raw XSS payload could
	 * still abuse the same-domain httponly cookie (documented residual risk).
	 */
	import { BROWSER } from 'esm-env'
	import { OpenAPI } from '$lib/gen'
	import { page } from '$app/state'
	import { onDestroy, onMount, setContext, type Snippet } from 'svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { base } from '$app/paths'
	import Login from '$lib/components/Login.svelte'

	type EmbedToken = { token?: string | null; public_app_domain?: string | null }

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
		if (e.source !== window.parent) return
		if (expectedEmbedderOrigin && e.origin !== expectedEmbedderOrigin) return
		if (e.data?.type === 'wm_embed_token') {
			const token = e.data.token ?? undefined
			viewerToken = token
			// The bearer token (when present) takes precedence over any cookie in
			// the backend auth, so this is the credential the app will use.
			OpenAPI.TOKEN = token
			viewerReady = true
			onViewerReady?.(token, requestTokenRefresh)
		}
	}

	/** Passed to the viewer snippet: when the rendered app gets a 401 (e.g. the
	 * embed token expired) it calls this to ask the embedder for a fresh token. */
	function requestTokenRefresh() {
		viewerReady = false
		window.parent.postMessage({ type: 'wm_embed_unauthorized' }, expectedEmbedderOrigin ?? '*')
	}

	// ---------------------------- embedder mode ----------------------------
	let status: 'loading' | 'ready' | 'noPermission' | 'notExists' = $state('loading')
	let embedToken: string | null = $state(null)
	let publicAppDomain: string | null = $state(null)
	let iframeEl: HTMLIFrameElement | undefined = $state(undefined)

	const iframeOrigin = $derived(
		publicAppDomain ? `${window.location.protocol}//${publicAppDomain}` : window.location.origin
	)

	function buildViewerUrl(): string {
		const url = new URL(window.location.href)
		url.searchParams.set(EMBED_PARAM, '1')
		url.searchParams.set(ORIGIN_PARAM, window.location.origin)
		// `pathname` already includes the SvelteKit base path; only swap the origin.
		return iframeOrigin + url.pathname + url.search + url.hash
	}

	async function initEmbedder() {
		status = 'loading'
		try {
			const resp = await fetchEmbedToken()
			embedToken = resp.token ?? null
			publicAppDomain = resp.public_app_domain ?? null
			status = 'ready'
			// If the iframe already loaded (re-mint case), push the fresh token.
			postTokenToIframe()
		} catch (e: any) {
			status = e?.status === 401 ? 'noPermission' : 'notExists'
		}
	}

	function postTokenToIframe() {
		iframeEl?.contentWindow?.postMessage(
			{ type: 'wm_embed_token', token: embedToken },
			iframeOrigin
		)
	}

	function handleEmbedderMessage(e: MessageEvent) {
		if (e.source !== iframeEl?.contentWindow) return
		if (e.origin !== iframeOrigin) return
		if (e.data?.type === 'wm_embed_ready') {
			postTokenToIframe()
		} else if (e.data?.type === 'wm_embed_unauthorized') {
			initEmbedder()
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
	<!-- Login happens here, on the main (embedder) domain, so the session cookie
	     is set on the main domain only and never reaches the iframe origin. -->
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
		sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals"
		allow="clipboard-read; clipboard-write; fullscreen"
	></iframe>
{/if}
