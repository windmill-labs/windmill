<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { getContext, onMount, untrack } from 'svelte'
	import { unsandboxedRawAppHtml } from './utils'
	import RawAppRecorderControls from './RawAppRecorderControls.svelte'

	interface Props {
		workspace: string
		user: UserExt | undefined
		secret: string | undefined
		path: string
		runnables: Record<string, Runnable>
		/** Offer the session recorder (in-workspace viewer only — it reads the
		 * bundle's DOM, which is only possible on the unsandboxed path). */
		recordable?: boolean
	}

	let { workspace, user, secret, path, runnables, recordable = false }: Props = $props()

	let iframe = $state() as HTMLIFrameElement | undefined

	// Get initial hash from parent URL to pass to the iframe
	let initialHash = ''

	// WIN-2006: unless the publisher opted into sandbox isolation, run the bundle
	// same-origin with full access (the default); otherwise the opaque-origin sandbox.
	const unsandboxedCtx = getContext<{ value: boolean }>('IS_APP_UNSANDBOXED')
	let unsandboxed = $derived(unsandboxedCtx?.value ?? false)
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
	//   (no cookie, no window.parent, no token). Root-relative so it resolves
	//   against the real host even when this component itself runs inside an opaque
	//   viewer (where `location.origin` is "null"). Context is handed over via
	//   postMessage — never baked into the document, never a credential.
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
		// backend adds the header when the flag is present.
		const coep = new URLSearchParams(window.location.search).has('wm_coep') ? '?wm_coep=1' : ''
		return `/api/w/${workspace}/apps_u/get_data/v/${secret}.html${coep}`
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

	function respondCtx() {
		iframe?.contentWindow?.postMessage(
			{
				type: 'windmill:ctx',
				// Same shape as the unsandboxed wrapper: always the object, so
				// `window.ctx.workspace` works for anonymous viewers too.
				ctx: { ctx: user, workspace },
				initialHash,
				storage: { local: bundleStorage ?? {}, session: {} }
			},
			'*'
		)
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
						respondCtx()
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
					respondCtx()
				}
				return
			}
			// Everything else must come from the bundle iframe.
			if (event.source !== iframe?.contentWindow) return
			if (data?.type === 'windmill:ready') {
				// Hand the bundle its context + shared storage before it evaluates.
				if (!framed) {
					bundleStorage = readDirect()
					respondCtx()
				} else if (bundleStorage !== undefined) {
					respondCtx()
				} else {
					pendingReady = true
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
	{#if recordable && unsandboxed}
		<RawAppRecorderControls {iframe} {workspace} {path} />
	{/if}
{/if}
