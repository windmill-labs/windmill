<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { getContext, onMount } from 'svelte'

	interface Props {
		workspace: string
		user: UserExt | undefined
		secret: string | undefined
		path: string
		runnables: Record<string, Runnable>
	}

	let { workspace, user, secret, path, runnables }: Props = $props()

	let iframe = $state() as HTMLIFrameElement | undefined

	// Get initial hash from parent URL to pass to the iframe
	let initialHash = ''

	// WIN-2006: the bundle is served from a real API URL as a sandboxed,
	// opaque-origin document (`CSP: sandbox` response header + the iframe sandbox
	// attribute), so a malicious bundle can never reach the authenticated
	// Windmill origin (no cookie, no window.parent, no token). The URL is
	// root-relative so it resolves against the real host even when this component
	// itself runs inside an opaque viewer (where `location.origin` is "null").
	// The user context is handed over via postMessage — never baked into the
	// document and never a credential.
	let iframeSrc = $derived(
		secret ? `/api/w/${workspace}/apps_u/get_data/v/${secret}.html` : undefined
	)

	// WIN-2006: when the publisher disabled sandbox isolation (and the viewer
	// consented upstream in PublicAppFrame), run the bundle same-origin with full
	// access — the backend serves the same wrapper URL without the `CSP: sandbox`
	// header, and we add `allow-same-origin` so relative `fetch('/api/...')` and
	// the session cookie work. Otherwise the default opaque-origin sandbox.
	const unsandboxedCtx = getContext<{ value: boolean }>('IS_APP_UNSANDBOXED')
	let unsandboxed = $derived(unsandboxedCtx?.value ?? false)
	let sandboxAttr = $derived(
		unsandboxed
			? 'allow-scripts allow-same-origin allow-forms allow-popups allow-downloads allow-modals allow-top-navigation'
			: 'allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals allow-top-navigation'
	)

	// Persistence for the bundle's (opaque-origin) localStorage, backed by a single
	// store shared across all apps. In the logged-in workspace viewer this runs on
	// the real origin, so it reads/writes real localStorage directly. Inside the
	// public embed viewer (opaque) there is no real storage, so it relays per-key
	// ops up to the embedder, the persistence authority. `framed` distinguishes the
	// two; the shared snapshot is handed to the bundle before it evaluates so its
	// localStorage is hydrated synchronously.
	const framed = typeof window !== 'undefined' && window.parent !== window
	const SHARED_LS_KEY = 'wm_apps_localstorage'
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
				ctx: user ? { ctx: user, workspace } : undefined,
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

<RawAppBackgroundRunner {workspace} editor={false} {iframe} {runnables} {path} />

{#if iframeSrc}
	<!-- `unsandboxed` (publisher disabled isolation + viewer consented) adds
	     allow-same-origin; the backend then omits the CSP: sandbox header so the
	     bundle runs same-origin with full access. -->
	<iframe
		bind:this={iframe}
		title="raw-app"
		src={iframeSrc}
		sandbox={sandboxAttr}
		class="w-full h-full min-h-screen bg-white border-none"
	></iframe>
{/if}
