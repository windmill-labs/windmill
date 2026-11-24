<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { Copy, Download, ExternalLink, X, ZoomIn, ZoomOut } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { on } from 'svelte/events'
	import { sendUserToast } from '$lib/toast'
	import Button from '../button/Button.svelte'
	import Disposable from '../drawer/Disposable.svelte'
	import { overlayHostActive, overlayShown, setTopmostSurface } from '../overlayHost.svelte'

	interface Props {
		src: string
		alt?: string
		/** Caption in the header; defaults to `alt`. Also names the downloaded file when it
		 *  ends in an extension (an S3 key, an attachment's file name). */
		title?: string
		isOpen: boolean
	}

	let { src, alt = '', title, isOpen = $bindable() }: Props = $props()

	const caption = $derived(title || alt || 'Image')

	const ZOOM_STEPS = [0.1, 0.25, 0.5, 0.75, 1, 1.5, 2, 3, 4, 6, 8]
	const MAX_SCALE = 8
	// Room kept around a fitted image: the top bar overlays the stage, so the image clears it.
	const FIT_MARGIN_X = 48
	const FIT_MARGIN_Y = 128

	let natural = $state<{ w: number; h: number } | undefined>(undefined)
	let stageW = $state(0)
	let stageH = $state(0)
	let stage: HTMLDivElement | undefined = $state(undefined)
	let imgEl: HTMLImageElement | undefined = $state(undefined)

	/** `fit` follows the viewport as it resizes; a number is a fixed scale of the natural size. */
	let zoom: number | 'fit' = $state('fit')
	let pan = $state({ x: 0, y: 0 })
	/** Eases button and keyboard zooms; wheel and drag must track the pointer without lag. */
	let animate = $state(false)

	const fitScale = $derived(
		natural && stageW && stageH
			? Math.min(1, (stageW - FIT_MARGIN_X) / natural.w, (stageH - FIT_MARGIN_Y) / natural.h)
			: 1
	)
	const scale = $derived(zoom === 'fit' ? fitScale : zoom)
	const minScale = $derived(Math.min(fitScale, ZOOM_STEPS[0]))
	const pannable = $derived(
		!!natural && (natural.w * scale > stageW + 1 || natural.h * scale > stageH + 1)
	)

	let disposable: Disposable | undefined = $state(undefined)
	const hostActive = overlayHostActive()
	const shown = overlayShown()
	setTopmostSurface(() => disposable?.isTopmost() ?? true)

	function onOpen() {
		// A bound `open` alone does not put the viewer on the disposable stack, and the stack
		// lifts it above the drawer or dialog it was opened from and gives Escape to the topmost.
		disposable?.openDrawer()
		zoom = 'fit'
		pan = { x: 0, y: 0 }
		animate = false
	}

	function close() {
		isOpen = false
	}

	function clampPan(p: { x: number; y: number }, s: number) {
		if (!natural) return { x: 0, y: 0 }
		const maxX = Math.max(0, (natural.w * s - stageW) / 2)
		const maxY = Math.max(0, (natural.h * s - stageH) / 2)
		return {
			x: Math.min(maxX, Math.max(-maxX, p.x)),
			y: Math.min(maxY, Math.max(-maxY, p.y))
		}
	}

	/** `at` is relative to the stage centre; that point stays under the pointer while zooming. */
	function zoomTo(next: number, at = { x: 0, y: 0 }) {
		const s = Math.min(MAX_SCALE, Math.max(minScale, next))
		const k = s / scale
		pan = clampPan({ x: at.x - (at.x - pan.x) * k, y: at.y - (at.y - pan.y) * k }, s)
		zoom = s
	}

	function fit() {
		animate = true
		zoom = 'fit'
		pan = { x: 0, y: 0 }
	}

	function zoomIn() {
		animate = true
		zoomTo(ZOOM_STEPS.find((z) => z > scale * 1.01) ?? MAX_SCALE)
	}

	function zoomOut() {
		animate = true
		zoomTo(ZOOM_STEPS.findLast((z) => z < scale / 1.01) ?? minScale)
	}

	function stagePoint(e: { clientX: number; clientY: number }) {
		const r = stage!.getBoundingClientRect()
		return { x: e.clientX - r.left - r.width / 2, y: e.clientY - r.top - r.height / 2 }
	}

	function toggleZoom(e: MouseEvent) {
		// The stage captures the pointer, so the double click targets the stage, not the image.
		const r = imgEl?.getBoundingClientRect()
		if (
			!r ||
			e.clientX < r.left ||
			e.clientX > r.right ||
			e.clientY < r.top ||
			e.clientY > r.bottom
		)
			return
		animate = true
		if (zoom === 'fit' || Math.abs(scale - fitScale) < 0.001) {
			zoomTo(fitScale < 1 ? 1 : 2, stagePoint(e))
		} else {
			fit()
		}
	}

	function wheelZoom(node: HTMLElement) {
		// Registered by hand: the default must be prevented, so the listener cannot be passive.
		return on(
			node,
			'wheel',
			(e: WheelEvent) => {
				e.preventDefault()
				animate = false
				const lines = e.deltaMode === 1 ? 33 : 1
				// Pinch on a trackpad arrives as ctrl+wheel with small deltas.
				const speed = e.ctrlKey ? 0.01 : 0.0015
				zoomTo(scale * Math.exp(-e.deltaY * lines * speed), stagePoint(e))
			},
			{ passive: false }
		)
	}

	let panning = $state(false)
	let drag:
		| { x: number; y: number; pan: { x: number; y: number }; moved: boolean; onImage: boolean }
		| undefined

	function onPointerDown(e: PointerEvent) {
		if (e.button !== 0) return
		drag = { x: e.clientX, y: e.clientY, pan, moved: false, onImage: e.target === imgEl }
		stage?.setPointerCapture(e.pointerId)
	}

	function onPointerMove(e: PointerEvent) {
		if (!drag) return
		const dx = e.clientX - drag.x
		const dy = e.clientY - drag.y
		if (!drag.moved && Math.hypot(dx, dy) < 4) return
		drag.moved = true
		if (pannable) {
			panning = true
			animate = false
			pan = clampPan({ x: drag.pan.x + dx, y: drag.pan.y + dy }, scale)
		}
	}

	/** A plain click beside the image dismisses; one that ends a pan does not. */
	let dismissOnClick = false

	function onPointerUp() {
		dismissOnClick = !!drag && !drag.moved && !drag.onImage
		drag = undefined
		panning = false
	}

	function onStageClick() {
		// Closed on `click`, not `pointerup`: outside-click guards elsewhere (the chat composer)
		// listen for this click and must still see the viewer open when it arrives.
		if (dismissOnClick) close()
		dismissOnClick = false
	}

	function onKeyDown(e: KeyboardEvent) {
		if (!isOpen || !hostActive() || !(disposable?.isTopmost() ?? true)) return
		if (e.metaKey || e.ctrlKey || e.altKey) return
		const action = {
			'+': zoomIn,
			'=': zoomIn,
			'-': zoomOut,
			'0': fit,
			'1': () => {
				animate = true
				zoomTo(1)
			}
		}[e.key]
		if (!action) return
		e.preventDefault()
		action()
	}

	const MIME_EXT: Record<string, string> = {
		'image/png': 'png',
		'image/jpeg': 'jpg',
		'image/gif': 'gif',
		'image/webp': 'webp',
		'image/svg+xml': 'svg',
		'image/avif': 'avif',
		'image/bmp': 'bmp'
	}

	/** The caption names the file, but the bytes decide its extension: chat attachments keep
	 *  their original name after being re-encoded (`diagram.svg` holding PNG bytes). */
	function fileName(mime: string) {
		const base = caption.split(/[/\\]/).pop() ?? ''
		const named = base.match(/^(.+)\.([a-z0-9]{2,5})$/i)
		const ext = MIME_EXT[mime] ?? named?.[2] ?? 'png'
		return `${named?.[1] ?? 'image'}.${ext}`
	}

	async function fetchBlob() {
		const res = await fetch(src)
		if (!res.ok) throw new Error(`HTTP ${res.status}`)
		return res.blob()
	}

	async function download() {
		try {
			const blob = await fetchBlob()
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = fileName(blob.type)
			a.click()
			setTimeout(() => URL.revokeObjectURL(url), 1000)
		} catch (e) {
			sendUserToast(`Could not download the image: ${e}`, true)
		}
	}

	async function openInNewTab() {
		if (!src.startsWith('data:')) {
			window.open(src, '_blank', 'noopener')
			return
		}
		// Browsers refuse to open a data: URL as a page; a blob URL of the same bytes opens.
		try {
			const url = URL.createObjectURL(await fetchBlob())
			window.open(url, '_blank', 'noopener')
			setTimeout(() => URL.revokeObjectURL(url), 60_000)
		} catch (e) {
			sendUserToast(`Could not open the image: ${e}`, true)
		}
	}

	async function copyImage() {
		try {
			// The clipboard takes PNG in every browser, other types in few: re-encode through a canvas.
			const canvas = document.createElement('canvas')
			canvas.width = imgEl!.naturalWidth
			canvas.height = imgEl!.naturalHeight
			canvas.getContext('2d')!.drawImage(imgEl!, 0, 0)
			const blob = new Promise<Blob>((resolve, reject) =>
				canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('encoding failed'))), 'image/png')
			)
			// Handed over as a promise so `write` runs within the click: Safari refuses it once
			// the user gesture has expired, which an awaited encode would let happen.
			await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })])
			sendUserToast('Image copied to clipboard')
		} catch (e) {
			sendUserToast(`Could not copy the image: ${e}`, true)
		}
	}
</script>

<svelte:window onkeydown={onKeyDown} />

{#if shown()}
	<Disposable
		bind:open={isOpen}
		bind:this={disposable}
		{onOpen}
		onClose={() => disposable?.closeDrawer()}
	>
		{#snippet children({ zIndex })}
			{#if isOpen}
				<Portal name="always-mounted">
					<div
						role="dialog"
						aria-modal="true"
						aria-label={caption}
						class="fixed inset-0 bg-black/85"
						style="z-index: {zIndex}"
						transition:fade|local={{ duration: 150 }}
					>
						<div
							bind:this={stage}
							bind:clientWidth={stageW}
							bind:clientHeight={stageH}
							{@attach wheelZoom}
							class="absolute inset-0 overflow-hidden touch-none select-none"
							class:cursor-grab={pannable && !panning}
							class:cursor-grabbing={panning}
							role="presentation"
							onpointerdown={onPointerDown}
							onpointermove={onPointerMove}
							onpointerup={onPointerUp}
							onclick={onStageClick}
							onpointercancel={() => {
								drag = undefined
								panning = false
							}}
							ondblclick={toggleZoom}
						>
							<img
								bind:this={imgEl}
								{src}
								{alt}
								draggable="false"
								class="absolute left-1/2 top-1/2 max-w-none {natural ? '' : 'opacity-0'} {animate
									? 'transition-transform duration-150 ease-out'
									: ''}"
								style="width: {natural?.w ?? 0}px; height: {natural?.h ??
									0}px; transform: translate(-50%, -50%) translate({pan.x}px, {pan.y}px) scale({scale});"
								onload={(e) => {
									const el = e.currentTarget as HTMLImageElement
									// An SVG without width/height has no intrinsic size.
									natural = { w: el.naturalWidth || 1024, h: el.naturalHeight || 768 }
								}}
							/>
						</div>

						<div
							class="absolute inset-x-0 top-0 flex items-center justify-between gap-3 p-3 pointer-events-none"
						>
							<div
								class="flex items-baseline gap-2 min-w-0 h-9 px-3 leading-9 rounded-md bg-surface-tertiary shadow-lg pointer-events-auto"
							>
								<span class="truncate text-xs font-semibold text-emphasis">{caption}</span>
								{#if natural}
									<span class="shrink-0 text-2xs text-secondary tabular-nums">
										{natural.w} × {natural.h}
									</span>
								{/if}
							</div>
							<div
								class="flex shrink-0 items-center gap-0.5 h-9 px-1 rounded-md bg-surface-tertiary shadow-lg pointer-events-auto"
							>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: ZoomOut }}
									title="Zoom out (−)"
									disabled={scale <= minScale + 0.001}
									onclick={zoomOut}
								/>
								<Button
									variant="subtle"
									unifiedSize="sm"
									btnClasses="w-14 justify-center tabular-nums"
									title="Fit to screen (0)"
									onclick={fit}
								>
									{Math.round(scale * 100)}%
								</Button>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: ZoomIn }}
									title="Zoom in (+)"
									disabled={scale >= MAX_SCALE - 0.001}
									onclick={zoomIn}
								/>
								<div class="w-px h-4 mx-1 bg-border-light"></div>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: Copy }}
									title="Copy image"
									onclick={copyImage}
								/>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: Download }}
									title="Download"
									onclick={download}
								/>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: ExternalLink }}
									title="Open in new tab"
									onclick={openInNewTab}
								/>
								<div class="w-px h-4 mx-1 bg-border-light"></div>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: X }}
									title="Close (Esc)"
									onclick={close}
								/>
							</div>
						</div>
					</div>
				</Portal>
			{/if}
		{/snippet}
	</Disposable>
{/if}
