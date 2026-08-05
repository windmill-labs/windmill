<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import { ClipboardCopy, Download, Maximize2, Minus, Plus, RotateCcw } from 'lucide-svelte'
	import { getOverlayHost } from '$lib/components/common/overlayHost.svelte'
	import type { PanZoom } from 'panzoom'

	let { code }: { code: string } = $props()

	const isDarkMode = useIsDarkMode()

	let svg = $state<string | undefined>(undefined)
	// The exact source that produced `svg`. The diagram is only shown while this
	// still matches the current `code`, so a later edit that fails to parse falls
	// back to the raw source instead of leaving a stale, mismatched diagram.
	let renderedCode = $state<string | undefined>(undefined)
	// Monotonic token so an earlier-started render that resolves late can't
	// overwrite the result of a newer one (out-of-order async on rapid code/theme changes).
	let renderSeq = 0

	let expanded = $state(false)

	// The dialog fills the enclosing pane when there is one (see overlayHost) and the viewport
	// otherwise, so a viewport-relative canvas height overflows it in any pane shorter than the
	// window. Size against whichever box it actually fills, less the chrome Modal.svelte wraps
	// the canvas in — read off that component, and only correct while it stays in step:
	// scroll wrapper p-4, box px-4/pt-5/pb-4 (sm:p-6) + sm:my-8, title row, and the body's mt-4.
	const CHROME_PX = { belowSm: 112, fromSm: 188 }
	const SM_BREAKPOINT_PX = 640
	const MIN_CANVAS_PX = 240
	const overlayHost = getOverlayHost()
	let windowWidth = $state(0)
	let windowHeight = $state(0)
	// undefined means "no host to measure", which 0 does not: the panel is resized rather than
	// unmounted, so a collapsed host measures 0 and must stay 0 rather than fall back to the window.
	let hostHeight = $state<number | undefined>(undefined)

	$effect(() => {
		if (!expanded) return
		const host = overlayHost?.el()
		if (!host) return
		// Seed synchronously so the first frame isn't sized off the window while the observer's
		// initial callback is still pending.
		hostHeight = host.clientHeight
		const observer = new ResizeObserver(() => (hostHeight = host.clientHeight))
		observer.observe(host)
		return () => {
			observer.disconnect()
			hostHeight = undefined
		}
	})

	// windowHeight covers the un-hosted case: documentElement's box is content-driven on a
	// scrollable page, so observing it would miss a purely vertical window resize.
	let canvasHeight = $derived(
		Math.max(
			MIN_CANVAS_PX,
			(hostHeight ?? windowHeight) -
				(windowWidth >= SM_BREAKPOINT_PX ? CHROME_PX.fromSm : CHROME_PX.belowSm)
		)
	)

	// Past this, render with whatever metrics are available rather than sit on the raw source.
	const FONT_LOAD_TIMEOUT_MS = 2000

	/**
	 * Mermaid sizes every node by measuring its rendered label, so the fonts that label will be
	 * drawn in must be loaded first: measured against fallback metrics the boxes come out too
	 * narrow and the labels overflow them once the real font swaps in.
	 *
	 * Only the non-ASCII of the source is offered to the emoji font. Its subsets cover digits,
	 * `#` and `*`, so passing the whole source would pull 64 KB for any diagram carrying a
	 * number. Keycaps still resolve: their U+20E3 is non-ASCII and shares a subset with the
	 * digit bases, and every other emoji class — flags, skin tones, ZWJ sequences — is
	 * non-ASCII by construction, so this needs no emoji grammar to keep up to date.
	 */
	async function loadLabelFonts(source: string): Promise<void> {
		// eslint-disable-next-line no-control-regex
		const nonAscii = source.replace(/[\x00-\x7F]/g, '')
		const loads = [document.fonts?.load('16px Inter', source)]
		if (nonAscii) loads.push(document.fonts?.load('16px "Noto Color Emoji"', nonAscii))
		let timer: ReturnType<typeof setTimeout> | undefined
		try {
			// Neither a rejection nor a stalled fetch may reach the caller's catch, which reads any
			// throw as a parse failure and drops the diagram to raw source.
			await Promise.race([
				Promise.all(loads).catch(() => {}),
				new Promise((resolve) => (timer = setTimeout(resolve, FONT_LOAD_TIMEOUT_MS)))
			])
		} finally {
			clearTimeout(timer)
		}
	}

	async function render(source: string, dark: boolean) {
		const seq = ++renderSeq
		if (!source?.trim()) {
			svg = undefined
			renderedCode = undefined
			return
		}
		try {
			const mermaid = (await import('mermaid')).default
			mermaid.initialize({
				startOnLoad: false,
				theme: dark ? 'dark' : 'default',
				securityLevel: 'strict',
				// Mermaid writes this stack into a <style> block inside the SVG, so the app's font
				// never reaches a diagram: the bundled vector emoji font has to be named here or
				// emoji fall back to the platform bitmap one, which ignores the zoom transform
				// (app.css). Inter sits last before it because the emoji font also covers digits,
				// # and *; on a box without the MS core fonts those would otherwise render as its
				// keycap glyphs. Keeping Inter behind them leaves diagram typography unchanged.
				fontFamily: '"trebuchet ms", verdana, arial, Inter, "Noto Color Emoji", sans-serif',
				// Throw on parse errors instead of injecting an orphan error diagram into the DOM.
				suppressErrorRendering: true
			})
			await loadLabelFonts(source)
			if (seq !== renderSeq) return
			// mermaid.render needs a fresh element id per attempt to avoid id collisions.
			const result = await mermaid.render(`mermaid-${randomUUID()}`, source)
			if (seq !== renderSeq) return
			svg = result.svg
			renderedCode = source
		} catch {
			// Parse failure (often a partial block still streaming in): fall back to the
			// raw source. `showSvg` already hides any previous diagram since `renderedCode`
			// no longer matches the current `code`.
		}
	}

	$effect(() => {
		void render(code, isDarkMode.val)
	})

	// Only show the diagram while it corresponds to the current source.
	let showSvg = $derived(svg !== undefined && renderedCode === code)

	function downloadSvg() {
		if (svg) {
			download('mermaid.svg', svg, 'image/svg+xml')
		}
	}

	// Pan/zoom is only wired up inside the fullscreen modal so the inline preview
	// stays a plain, non-interactive thumbnail.
	let panzoomInstance = $state<PanZoom | undefined>(undefined)
	let panzoomNode: HTMLElement | undefined = undefined

	function panzoomAction(node: HTMLElement) {
		let instance: PanZoom | undefined
		// Guard the async import against a close-before-resolve: without it,
		// destroy() (instance still undefined) disposes nothing and the late
		// .then() would build a leaked panzoom on an already-detached node.
		let disposed = false
		panzoomNode = node
		import('panzoom').then(({ default: panzoom }) => {
			if (disposed) return
			instance = panzoom(node, {
				bounds: true,
				boundsPadding: 0.1,
				maxZoom: 8,
				minZoom: 0.2,
				zoomDoubleClickSpeed: 1,
				smoothScroll: false
			})
			panzoomInstance = instance
		})
		return {
			destroy() {
				disposed = true
				instance?.dispose()
				panzoomInstance = undefined
				panzoomNode = undefined
			}
		}
	}

	function zoomBy(ratio: number) {
		if (!panzoomInstance || !panzoomNode) return
		// smoothZoom anchors on client coordinates — zoom around the viewport center.
		const rect = panzoomNode.getBoundingClientRect()
		panzoomInstance.smoothZoom(rect.left + rect.width / 2, rect.top + rect.height / 2, ratio)
	}

	function resetZoom() {
		if (!panzoomInstance) return
		panzoomInstance.moveTo(0, 0)
		panzoomInstance.zoomAbs(0, 0, 1)
	}
</script>

<svelte:window bind:innerHeight={windowHeight} bind:innerWidth={windowWidth} />

{#if showSvg}
	<div class="relative">
		<div class="absolute top-2 right-2 z-20 flex flex-row gap-1">
			<Button
				onclick={() => copyToClipboard(code)}
				color="light"
				size="xs2"
				startIcon={{ icon: ClipboardCopy }}
				iconOnly
				title="Copy diagram source"
			/>
			<Button
				onclick={downloadSvg}
				color="light"
				size="xs2"
				startIcon={{ icon: Download }}
				iconOnly
				title="Download as SVG"
			/>
			<Button
				onclick={() => (expanded = true)}
				color="light"
				size="xs2"
				startIcon={{ icon: Maximize2 }}
				iconOnly
				title="Expand and zoom"
			/>
		</div>
		<div class="p-2 flex justify-center overflow-x-auto">
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html svg}
		</div>
	</div>

	<Modal bind:open={expanded} title="Diagram" kind="X" class="sm:max-w-none w-[92vw]">
		{#snippet settings()}
			<div class="flex flex-row gap-1 mr-8">
				<Button
					onclick={() => zoomBy(1 / 1.3)}
					color="light"
					size="xs2"
					startIcon={{ icon: Minus }}
					iconOnly
					title="Zoom out"
				/>
				<Button
					onclick={() => zoomBy(1.3)}
					color="light"
					size="xs2"
					startIcon={{ icon: Plus }}
					iconOnly
					title="Zoom in"
				/>
				<Button
					onclick={resetZoom}
					color="light"
					size="xs2"
					startIcon={{ icon: RotateCcw }}
					iconOnly
					title="Reset zoom"
				/>
				<Button
					onclick={downloadSvg}
					color="light"
					size="xs2"
					startIcon={{ icon: Download }}
					iconOnly
					title="Download as SVG"
				/>
			</div>
		{/snippet}
		<div
			class="relative w-full overflow-hidden rounded border cursor-grab bg-surface-secondary"
			style="height: {canvasHeight}px"
		>
			{#if expanded}
				<div use:panzoomAction class="w-full h-full flex items-center justify-center">
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html svg}
				</div>
			{/if}
		</div>
	</Modal>
{:else}
	<!-- Fallback while loading or when rendering fails: show the raw source -->
	<pre class="overflow-auto max-h-screen text-xs p-2">{code}</pre>
{/if}
