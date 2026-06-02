<script lang="ts">
	import { type UserExt } from '$lib/stores'
	import RawAppBackgroundRunner from './RawAppBackgroundRunner.svelte'
	import type { Runnable } from './rawAppPolicy'
	import { onMount } from 'svelte'

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

	// Persistence for the bundle's (opaque-origin) localStorage. In the logged-in
	// workspace viewer this component runs on the real origin, so we persist
	// directly to real localStorage. Inside the public embed viewer (opaque) there
	// is no real storage, so we relay to the embedder, which is the persistence
	// authority. `framed` distinguishes the two; reads are served before the bundle
	// evaluates so the bundle sees its storage hydrated synchronously.
	const framed = typeof window !== 'undefined' && window.parent !== window
	const ns = `${workspace}:${path}`
	const rawKey = `wm_rawstore:${ns}`
	let bundleStorage: Record<string, string> | undefined = undefined
	let pendingReady = false

	function readDirect(): Record<string, string> {
		try {
			return JSON.parse(localStorage.getItem(rawKey) || '{}')
		} catch (_) {
			return {}
		}
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
			// Pre-fetch the persisted snapshot from the embedder.
			try {
				window.parent.postMessage({ type: 'wm_raw_ls_req', ns }, '*')
			} catch (_) {}
		}
	})

	$effect(() => {
		function handleMessage(event: MessageEvent) {
			const data = event.data
			// Persisted-storage hydration from the embedder (public mode only).
			if (
				framed &&
				event.source === window.parent &&
				data?.type === 'wm_raw_ls_set' &&
				data.ns === ns
			) {
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
				// Hand the bundle its context + persisted storage before it evaluates.
				if (!framed) {
					bundleStorage = readDirect()
					respondCtx()
				} else if (bundleStorage !== undefined) {
					respondCtx()
				} else {
					pendingReady = true
				}
			} else if (data?.type === 'wm_ls_sync') {
				// The bundle wrote to localStorage — persist the snapshot.
				if (!framed) {
					try {
						localStorage.setItem(rawKey, JSON.stringify(data.data || {}))
					} catch (_) {}
				} else {
					try {
						window.parent.postMessage({ type: 'wm_raw_ls_sync', ns, data: data.data }, '*')
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
	<iframe
		bind:this={iframe}
		title="raw-app"
		src={iframeSrc}
		sandbox="allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-downloads allow-modals allow-top-navigation"
		class="w-full h-full min-h-screen bg-white border-none"
	></iframe>
{/if}
