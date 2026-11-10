<script module lang="ts">
	import { randomUUID } from '$lib/utils/uuid'

	const MAX_REMEMBERED_DIAGRAMS = 50
	const STREAM_QUIET_MS = 300
	const remembered = new Map<string, { id: string; svg: string }>()

	function newDiagramId() {
		return `mermaid-${randomUUID()}`
	}

	function recall(key: string): string | undefined {
		const entry = remembered.get(key)
		if (!entry) return undefined
		remembered.delete(key)
		remembered.set(key, entry)
		// The id scopes the SVG's styles and arrow markers, and a `url(#id)` resolves to the first
		// match in the document: sharing it with a copy in a hidden preview tab loses the arrows.
		return entry.svg.replaceAll(entry.id, newDiagramId())
	}

	function remember(key: string, id: string, svg: string) {
		remembered.set(key, { id, svg })
		if (remembered.size > MAX_REMEMBERED_DIAGRAMS) {
			remembered.delete(remembered.keys().next().value!)
		}
	}
</script>

<script lang="ts">
	import { onDestroy } from 'svelte'
	import { slide } from 'svelte/transition'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { ClipboardCopy, Download, Maximize2, Minus, Plus, RotateCcw } from 'lucide-svelte'
	import type { PanZoom } from 'panzoom'

	let { code, forceLight = false }: { code: string; forceLight?: boolean } = $props()

	const isDarkMode = useIsDarkMode()
	const reducedMotion = useReducedMotion()

	let svg = $state<string | undefined>(undefined)
	// The latest finished render had nothing to draw: show the source rather than a diagram that
	// no longer matches it. Held across a re-render so partial blocks streaming in stay as text.
	let showSource = $state(false)
	// Monotonic token so an earlier-started render that resolves late can't
	// overwrite the result of a newer one (out-of-order async on rapid code/theme changes).
	let renderSeq = 0
	// False while a render is in flight, true once it drew or failed. Exposed as
	// data-mermaid-pending so a print can wait for diagrams without counting fences.
	let settled = $state(false)
	// Only the placeholder-to-diagram swap animates, and a print must not capture it midway.
	let revealMs = $state(0)

	let expanded = $state(false)

	async function render(source: string, dark: boolean) {
		const seq = ++renderSeq
		if (!source?.trim()) {
			svg = undefined
			revealMs = 0
			showSource = true
			settled = true
			return
		}
		const key = `${dark}:${source}`
		const known = recall(key)
		if (known !== undefined) {
			revealMs = 0
			svg = known
			showSource = false
			settled = true
			return
		}
		settled = false
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
			// mermaid.render needs a fresh element id per attempt to avoid id collisions.
			let id = newDiagramId()
			let result = await mermaid.render(id, source)
			if (seq !== renderSeq) return
			// Mermaid sizes each node by measuring its label, so a font still in flight yields
			// boxes cut to fallback metrics that the real glyphs then overflow. Drawing the
			// diagram is itself what asks for the fonts its glyphs need — including the emoji
			// subsets — so let them settle and lay it out again against the true metrics before
			// anything is shown.
			if (document.fonts?.status === 'loading') {
				await document.fonts.ready
				if (seq !== renderSeq) return
				id = newDiagramId()
				result = await mermaid.render(id, source)
				if (seq !== renderSeq) return
			}
			remember(key, id, result.svg)
			revealMs = showSource || forceLight || reducedMotion.val ? 0 : 200
			svg = result.svg
			showSource = false
		} catch {
			// Parse failure (often a partial block still streaming in): fall back to the raw source.
			if (seq === renderSeq) {
				revealMs = 0
				showSource = true
			}
		} finally {
			if (seq === renderSeq) settled = true
		}
	}

	let quietTimer: ReturnType<typeof setTimeout> | undefined

	// Partial text alternates between parsing and not, so drawing each chunk of a stream flips
	// the block between text and diagram: a change landing soon after another stays text until
	// the source goes quiet. A lone change (an edit, a version switch) still draws at once.
	$effect(() => {
		const source = code
		const dark = !forceLight && isDarkMode.val
		const burst = quietTimer !== undefined
		clearTimeout(quietTimer)
		quietTimer = setTimeout(() => {
			quietTimer = undefined
			if (burst) void render(source, dark)
		}, STREAM_QUIET_MS)
		if (burst) {
			renderSeq++
			revealMs = 0
			showSource = true
			settled = false
		} else {
			void render(source, dark)
		}
	})

	onDestroy(() => clearTimeout(quietTimer))

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

{#if showSource}
	<pre class="overflow-auto max-h-screen text-xs p-2" data-mermaid-pending={!settled}>{code}</pre>
{:else if svg === undefined}
	<!-- Outgoing slide paired with the diagram's incoming one, so the block's height eases from
	     this to the diagram's instead of jumping and shoving the document below it. -->
	<div class="p-2" data-mermaid-pending="true" out:slide={{ duration: revealMs }}>
		<Skeleton layout={[[10]]} />
	</div>
{:else}
	<div class="relative" data-mermaid-pending={!settled} in:slide={{ duration: revealMs }}>
		<div class="absolute top-2 right-2 z-20 flex flex-row gap-1 print:hidden">
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

	<Modal bind:open={expanded} title="Diagram" kind="X" fillHeight class="sm:max-w-none w-[92vw]">
		{#snippet settings()}
			<div class="flex flex-row gap-1">
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
			class="relative w-full h-full overflow-hidden rounded border cursor-grab bg-surface-secondary"
		>
			{#if expanded}
				<div use:panzoomAction class="w-full h-full flex items-center justify-center">
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html svg}
				</div>
			{/if}
		</div>
	</Modal>
{/if}
