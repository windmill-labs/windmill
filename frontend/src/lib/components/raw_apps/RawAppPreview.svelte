<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { getContext, onMount, untrack } from 'svelte'
	import { unsandboxedRawAppHtml } from './utils'
	import { randomSecret } from '$lib/utils/uuid'

	// Per-mount secret proving a `windmill:ready` came from the document we loaded.
	// It is the only thing authenticating that document — a port proves nothing,
	// since any sender can transfer one — so it must not be guessable.
	const handshakeNonce = randomSecret()

	interface Props {
		workspace: string
		user: UserExt | undefined
		secret: string | undefined
		path: string
		runnables: Record<string, Runnable>
		/** Called with the bundle's iframe once it is mounted. The session recorder
		 * (publish flow) needs it to read the app's DOM, which is only possible on
		 * the unsandboxed path. */
		oniframe?: (iframe: HTMLIFrameElement | undefined) => void
	}

	let { workspace, user, secret, path, runnables, oniframe }: Props = $props()

	$effect(() => {
		const el = unsandboxed ? iframe : undefined
		oniframe?.(el)
		// Withdraw it on teardown: a consumer that keeps the reference (the session
		// recorder) would otherwise go on addressing a document that is gone.
		return () => oniframe?.(undefined)
	})

	let iframe = $state() as HTMLIFrameElement | undefined

	// Get initial hash from parent URL to pass to the iframe
	let initialHash = ''

	// WIN-2006: unless the publisher opted into sandbox isolation, run the bundle
	// same-origin with full access (the default); otherwise the opaque-origin sandbox.
	const unsandboxedCtx = getContext<{ value: boolean }>('IS_APP_UNSANDBOXED')
	let unsandboxed = $derived(unsandboxedCtx?.value ?? false)

	// Viewer-scoped frontend SDK token (PublicAppFrame mints it after the viewer
	// consents). Sandbox-only, so it reaches the bundle through the handshake
	// below — the unsandboxed wrapper never carries a credential.
	const sdkTokenCtx = getContext<{ value: string | undefined }>('RAW_APP_SDK_TOKEN')
	// Unsandboxed (the default) must match the pre-isolation viewer exactly: NO
	// sandbox attribute (a same-origin blob with full session — an attribute would
	// only break leftover features like unsandboxed popups for OAuth flows, while
	// adding no isolation). The sandboxed path keeps the restrictive attribute; the
	// wrapper document's `CSP: sandbox` response header enforces the opaque origin
	// regardless.
	let sandboxAttr = $derived(
		unsandboxed
			? undefined
			: 'allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals allow-top-navigation'
	)

	// WIN-2006: source of the bundle iframe.
	// - DEFAULT (isolated): a real API URL serving a sandboxed, opaque-origin
	//   document (`CSP: sandbox` response header + the iframe sandbox attribute),
	//   so a malicious bundle can never reach the authenticated Windmill origin
	//   (no cookie, no window.parent). Root-relative so it resolves against the
	//   real host even when this component itself runs inside an opaque viewer
	//   (where `location.origin` is "null"). Context — and, when the viewer
	//   approved SDK scopes, the viewer-scoped token — is handed over via
	//   postMessage, never baked into the document.
	// - UNSANDBOXED (the default — publisher did not opt into isolation): a
	//   client-built blob: wrapper (same-origin with the SPA) loaded with `allow-same-origin`,
	//   so relative `fetch('/api/...')` and the session cookie work. The backend
	//   `.html` is ALWAYS sandboxed, so we must build the same-origin wrapper here
	//   rather than relax a real-origin endpoint a victim could be linked to.
	let iframeSrc = $derived.by(() => {
		if (!secret || typeof window === 'undefined') return undefined
		if (unsandboxed) {
			// untrack(user) so userStore refreshes don't regenerate the blob URL and
			// reload the iframe (losing state); ctx is only needed for initial render.
			// Always pass the wrapper object — pre-sandbox bundles rely on
			// `window.ctx.workspace` even for anonymous viewers (ctx.ctx undefined).
			const u = untrack(() => user)
			const html = unsandboxedRawAppHtml(
				workspace,
				secret,
				{ ctx: u, workspace },
				window.location.origin,
				window.location.hash || ''
			)
			return URL.createObjectURL(new Blob([html], { type: 'text/html' }))
		}
		// `wm_coep` (embed-in-cross-origin-isolated-page opt-in) must be propagated
		// to the wrapper document: under a COEP `require-corp` embedder, a nested
		// document is only allowed to load if it asserts COEP itself, so the
		// backend adds the header when the flag is present. Also request it when
		// this document is itself cross-origin isolated (e.g. the raw app editor)
		// — the wrapper would otherwise be blocked outright, URL flag or not.
		const coep =
			new URLSearchParams(window.location.search).has('wm_coep') || window.crossOriginIsolated
				? 'wm_coep=1&'
				: ''
		// wm_hs: the handshake nonce, readable only by the document we load here —
		// see `respondCtx` for what it gates.
		return `/api/w/${workspace}/apps_u/get_data/v/${secret}.html?${coep}wm_hs=${handshakeNonce}`
	})

	// Revoke blob: URLs (unsandboxed path) when they change or on unmount.
	$effect(() => {
		const url = iframeSrc
		return () => {
			if (url && url.startsWith('blob:')) URL.revokeObjectURL(url)
		}
	})

	// Persistence for the bundle's (opaque-origin) localStorage, backed by a store
	// scoped PER APP (keyed by workspace + app path) so one sandboxed app can't read
	// or clobber another's (even two apps at the same path in different workspaces). On a real origin (workspace viewer, public page — even when
	// that page sits inside someone else's iframe) it reads/writes real localStorage
	// directly. Only inside an opaque frame (the Windmill embed viewer), where Web
	// Storage throws, does it relay per-key ops up to the embedder, the persistence
	// authority. `framed` therefore probes storage rather than just `window.parent`:
	// an externally-embedded public page is framed too, but its parent is not the
	// Windmill embedder and would never answer the relay (leaving the bundle without
	// ctx). The snapshot is handed to the bundle before it evaluates so its
	// localStorage is hydrated synchronously.
	const SHARED_LS_KEY = `wm_apps_localstorage:${workspace}:${path}`
	function storageAccessible(): boolean {
		try {
			localStorage.getItem(SHARED_LS_KEY)
			return true
		} catch (_) {
			return false
		}
	}
	const framed = typeof window !== 'undefined' && window.parent !== window && !storageAccessible()
	let bundleStorage: Record<string, string> | undefined = undefined
	let pendingReady = false
	// The `windmill:ready` we could not answer yet (storage still loading). Holding
	// the port is what makes the deferred reply safe: it stays bound to the document
	// that sent it even if the frame is navigated while we wait.
	let pendingNonce: string | undefined = undefined
	let pendingPort: MessagePort | undefined = undefined

	function readDirect(): Record<string, string> {
		try {
			return JSON.parse(localStorage.getItem(SHARED_LS_KEY) || '{}')
		} catch (_) {
			return {}
		}
	}

	function applyDirectOp(d: any) {
		try {
			const s = readDirect()
			if (d.op === 'set') s[d.key] = String(d.value)
			else if (d.op === 'remove') delete s[d.key]
			else if (d.op === 'clear') for (const k in s) delete s[k]
			localStorage.setItem(SHARED_LS_KEY, JSON.stringify(s))
		} catch (_) {}
	}

	/** The nonce authenticates the asker: it lives in our URL, which a document
	 * navigated into the frame can't read. Replying on the port that document
	 * transferred then keeps the answer from landing in whatever document is
	 * active by the time we reply. Both are needed. */
	function respondCtx(nonceEcho?: string, port?: MessagePort) {
		// No port means the sender is not a wrapper we served — every one of ours
		// transfers one. Answering anyway would hand the viewer's identity to it.
		if (!port) return
		const sdkToken = nonceEcho === handshakeNonce ? sdkTokenCtx?.value : undefined
		const payload = {
			type: 'windmill:ctx',
			// Same shape as the unsandboxed wrapper: always the object, so
			// `window.ctx.workspace` works for anonymous viewers too.
			ctx: { ctx: user, workspace },
			initialHash,
			storage: { local: bundleStorage ?? {}, session: {} },
			// The wrapper turns this into `window.process.env` before it injects the
			// bundle, which is what a bundled `windmill-client` reads at module load.
			// `baseUrl` comes from here because the bundle's own origin is opaque.
			...(sdkToken ? { sdk: { token: sdkToken, baseUrl: window.location.origin, workspace } } : {})
		}
		port.postMessage(payload)
	}

	onMount(() => {
		initialHash = window.location.hash || ''
		if (framed) {
			// Pre-fetch the shared store from the embedder.
			try {
				window.parent.postMessage({ type: 'wm_ls_req' }, '*')
			} catch (_) {}
			// If the parent never answers (it isn't the Windmill embedder, e.g. an
			// opaque context created by a third party), don't hold the bundle's ctx
			// hostage: proceed with empty storage. Must beat the backend wrapper's
			// own 1.5s no-ctx fallback.
			const fallback = setTimeout(() => {
				if (bundleStorage === undefined) {
					bundleStorage = {}
					if (pendingReady) {
						pendingReady = false
						respondCtx(pendingNonce, pendingPort)
					}
				}
			}, 750)
			return () => clearTimeout(fallback)
		}
	})

	$effect(() => {
		function handleMessage(event: MessageEvent) {
			const data = event.data
			// Shared-store hydration from the embedder (public mode only).
			if (framed && event.source === window.parent && data?.type === 'wm_ls_hydrate') {
				bundleStorage = data.data || {}
				if (pendingReady) {
					pendingReady = false
					respondCtx(pendingNonce, pendingPort)
				}
				return
			}
			// Everything else must come from the bundle iframe.
			if (event.source !== iframe?.contentWindow) return
			if (data?.type === 'windmill:ready') {
				// Hand the bundle its context + shared storage before it evaluates.
				const nonceEcho = typeof data.nonce === 'string' ? data.nonce : undefined
				const port = event.ports?.[0]
				if (!framed) {
					bundleStorage = readDirect()
					respondCtx(nonceEcho, port)
				} else if (bundleStorage !== undefined) {
					respondCtx(nonceEcho, port)
				} else {
					pendingReady = true
					pendingNonce = nonceEcho
					pendingPort = port
				}
			} else if (data?.type === 'wm_ls_op') {
				// The bundle mutated localStorage — apply it to the shared store.
				if (!framed) {
					applyDirectOp(data)
				} else {
					try {
						window.parent.postMessage(
							{ type: 'wm_ls_op', op: data.op, key: data.key, value: data.value },
							'*'
						)
					} catch (_) {}
				}
			} else if (data?.type === 'windmill:hashchange') {
				// Keep the parent URL hash in sync for shareable URLs.
				const newHash = data.hash || ''
				if (window.location.hash !== newHash) {
					history.replaceState(null, '', newHash || window.location.pathname)
				}
			}
		}

		window.addEventListener('message', handleMessage)
		return () => window.removeEventListener('message', handleMessage)
	})
</script>

<RawAppBackgroundRunner
	{workspace}
	editor={false}
	{iframe}
	{runnables}
	{path}
	gateJobIds={!unsandboxed}
/>

{#if iframeSrc}
	<!-- `unsandboxed` (the default — publisher did not opt into isolation) adds
	     allow-same-origin and loads a same-origin blob: wrapper, so the bundle runs
	     with full access. The sandboxed path loads the always-CSP-sandboxed backend
	     wrapper, which stays opaque even on direct navigation. -->
	<!-- referrerpolicy (sandboxed only, for exact legacy parity): the hosting page
	     URL can carry a viewer credential (the JWT path segment of share links);
	     without this, the bundle document would see it via document.referrer. -->
	<iframe
		bind:this={iframe}
		title="raw-app"
		src={iframeSrc}
		sandbox={sandboxAttr}
		referrerpolicy={unsandboxed ? undefined : 'no-referrer'}
		class="w-full h-full min-h-screen bg-white border-none"
	></iframe>
{/if}
