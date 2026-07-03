<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import { ClipboardCopy, Download, Maximize2, Minus, Plus, RotateCcw } from 'lucide-svelte'
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
				// Throw on parse errors instead of injecting an orphan error diagram into the DOM.
				suppressErrorRendering: true
			})
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
			class="relative w-full h-[78vh] overflow-hidden rounded border cursor-grab bg-surface-secondary"
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
