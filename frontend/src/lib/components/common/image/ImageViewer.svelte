<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { Copy, Download, ExternalLink, Maximize, PanelRight, X } from 'lucide-svelte'
	import ZoomControl from './ZoomControl.svelte'
	import { useZoomPan } from './zoomPan.svelte'
	import { fade } from 'svelte/transition'
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
		/** Shows the image in a viewer elsewhere (a session's preview panel), closing this one. */
		onOpenInPreview?: () => void
	}

	let { src, alt = '', title, isOpen = $bindable(), onOpenInPreview }: Props = $props()

	const caption = $derived(title || alt || 'Image')

	/** A plain click beside the image dismisses; one that ends a pan does not. */
	let dismissOnClick = false
	const view = useZoomPan({
		// The top bar overlays the stage, so a fitted image keeps clear of it.
		margin: { x: 48, y: 128 },
		onTap: (onImage) => (dismissOnClick = !onImage)
	})

	let imgEl: HTMLImageElement | undefined = $state(undefined)
	let disposable: Disposable | undefined = $state(undefined)
	const hostActive = overlayHostActive()
	const shown = overlayShown()
	setTopmostSurface(() => disposable?.isTopmost() ?? true)

	function onOpen() {
		// A bound `open` alone does not put the viewer on the disposable stack, and the stack
		// lifts it above the drawer or dialog it was opened from and gives Escape to the topmost.
		disposable?.openDrawer()
		view.reset()
	}

	function close() {
		isOpen = false
	}

	function onStageClick() {
		// Closed on `click`, not `pointerup`: outside-click guards elsewhere (the chat composer)
		// listen for this click and must still see the viewer open when it arrives.
		if (dismissOnClick) close()
		dismissOnClick = false
	}

	function onKeyDown(e: KeyboardEvent) {
		if (!isOpen || !hostActive() || !(disposable?.isTopmost() ?? true)) return
		if (view.handleKey(e)) e.preventDefault()
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
		// The blob URL takes this page's origin, so only plain raster types open as they are;
		// anything else, SVG included since it opens as a document, is re-encoded to PNG.
		try {
			const blob = await fetchBlob()
			const url = URL.createObjectURL(RASTER_TYPES.has(blob.type) ? blob : await rasterize())
			window.open(url, '_blank', 'noopener')
			setTimeout(() => URL.revokeObjectURL(url), 60_000)
		} catch (e) {
			sendUserToast(`Could not open the image: ${e}`, true)
		}
	}

	const RASTER_TYPES = new Set([
		'image/png',
		'image/jpeg',
		'image/gif',
		'image/webp',
		'image/avif',
		'image/bmp'
	])

	/** The displayed image as PNG bytes, drawn through a canvas. */
	function rasterize(): Promise<Blob> {
		const canvas = document.createElement('canvas')
		canvas.width = natural?.w ?? imgEl!.naturalWidth
		canvas.height = natural?.h ?? imgEl!.naturalHeight
		canvas.getContext('2d')!.drawImage(imgEl!, 0, 0, canvas.width, canvas.height)
		return new Promise<Blob>((resolve, reject) =>
			canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('encoding failed'))), 'image/png')
		)
	}

	async function copyImage() {
		try {
			// The clipboard takes PNG in every browser, other types in few: re-encode through a canvas.
			const blob = rasterize()
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
							{@attach view.surface}
							class="absolute inset-0 overflow-hidden touch-none select-none"
							class:cursor-grab={view.pannable && !view.panning}
							class:cursor-grabbing={view.panning}
							role="presentation"
							onclick={onStageClick}
						>
							<img
								bind:this={imgEl}
								{@attach view.image}
								{src}
								{alt}
								draggable="false"
								class={view.imageClass}
								style={view.imageStyle}
							/>
						</div>

						<div
							class="absolute inset-x-0 top-0 flex items-center justify-between gap-3 p-3 pointer-events-none"
						>
							<div
								class="flex items-baseline gap-2 min-w-0 h-9 px-1 leading-9 drop-shadow pointer-events-auto"
							>
								<!-- The overlay is black in both themes, and a zoomed image can pass under the
								caption, hence fixed white text with a shadow rather than theme tokens. -->
								<span class="truncate text-xs font-medium text-white/90">{caption}</span>
								{#if view.natural}
									<span class="shrink-0 text-2xs text-white/60 tabular-nums">
										{view.natural.w} × {view.natural.h}
									</span>
								{/if}
							</div>
							<div
								class="flex shrink-0 items-center gap-0.5 h-9 px-1 rounded-md bg-surface-tertiary shadow-lg pointer-events-auto"
							>
								<ZoomControl
									scale={view.scale}
									min={view.minScale}
									max={view.maxScale}
									onZoomOut={view.zoomOut}
									onZoomIn={view.zoomIn}
									onZoomTo={view.zoomToAnimated}
								/>
								<Button
									variant="subtle"
									unifiedSize="sm"
									iconOnly
									startIcon={{ icon: Maximize }}
									title="Fit to screen (0)"
									onclick={view.fit}
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
								{#if onOpenInPreview}
									<Button
										variant="subtle"
										unifiedSize="sm"
										iconOnly
										startIcon={{ icon: PanelRight }}
										title="Open in preview"
										onclick={() => {
											close()
											onOpenInPreview?.()
										}}
									/>
								{/if}
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
