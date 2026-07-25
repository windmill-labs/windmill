<script lang="ts">
	import Modal from '../common/modal/Modal.svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		/** Inline preview iframe rendering the user's app (same-origin app-preview.html) */
		iframe: HTMLIFrameElement | undefined
	}

	let { iframe }: Props = $props()

	let open = $state(false)

	// One toast per remote origin — a page pulling many assets from one host
	// should not stack toasts.
	const warnedOrigins = new Set<string>()

	const RESOURCE_TAGS = new Set(['IMG', 'SCRIPT', 'LINK', 'AUDIO', 'VIDEO', 'SOURCE', 'IFRAME'])

	// The editor page is cross-origin isolated (COEP require-corp — required for
	// the SharedArrayBuffer-based TS workers and in-browser bundling), and COEP
	// forces every nested document, including the preview, to be isolated too. So
	// cross-origin resources without CORS/CORP headers are blocked in the preview
	// while loading fine on the deployed app: surface that as a warning instead
	// of a silently broken image.
	function onResourceError(e: Event) {
		// Elements belong to the preview window's realm, so no instanceof checks —
		// duck-type instead. Resource load errors don't bubble but do capture
		// through the window; runtime ErrorEvents target the window itself and are
		// skipped by the tagName check.
		const el = e.target as (Element & { currentSrc?: string; src?: string; href?: string }) | null
		if (!el?.tagName || !RESOURCE_TAGS.has(el.tagName)) return
		const raw = el.currentSrc || el.src || el.href
		if (typeof raw !== 'string' || !raw) return
		let url: URL
		try {
			url = new URL(raw)
		} catch {
			return
		}
		// Same-origin failures (plain 404s) and non-http(s) schemes are not COEP blocks.
		if (!/^https?:$/.test(url.protocol) || url.origin === window.location.origin) return
		if (warnedOrigins.has(url.origin)) return
		warnedOrigins.add(url.origin)
		sendUserToast(
			`The editor preview blocked a cross-origin resource (${url.host}) due to COEP/CORS isolation. It will load normally on the deployed app.`,
			'warning',
			[{ label: 'Read more', callback: () => (open = true) }],
			undefined,
			10000
		)
	}

	export function attachTo(win: Window | null | undefined) {
		win?.addEventListener('error', onResourceError, true)
	}

	$effect(() => {
		const el = iframe
		if (!el) return
		const attach = () => attachTo(el.contentWindow)
		el.addEventListener('load', attach)
		// The iframe may already be loaded when this mounts; addEventListener
		// dedupes the (handler, capture) pair, so the load event re-firing on the
		// same window cannot double-attach.
		if (el.contentDocument?.readyState === 'complete') attach()
		return () => el.removeEventListener('load', attach)
	})
</script>

<Modal bind:open title="Cross-origin resources in the editor preview" kind="X">
	<div class="flex flex-col gap-3">
		<p>
			The app editor runs in a <b>cross-origin isolated</b> context (COOP/COEP headers). This is
			required for <code>SharedArrayBuffer</code>, which powers the TypeScript language workers and
			lets the editor build and preview your frontend live in the browser.
		</p>
		<p>
			A side effect is that the browser refuses to load cross-origin resources (images, scripts,
			stylesheets, media…) unless the remote server explicitly opts in with CORS or a
			<code>Cross-Origin-Resource-Policy</code> header. Resources from servers that don't are
			blocked <b>in the editor preview only</b>.
		</p>
		<p>
			The deployed app is served without these headers, so the same resources load normally there —
			open the deployed app link to verify. If you control the remote server, sending
			<code>Cross-Origin-Resource-Policy: cross-origin</code> makes the resource load in the editor too.
		</p>
	</div>
</Modal>
