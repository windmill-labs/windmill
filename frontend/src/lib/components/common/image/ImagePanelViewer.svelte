<script lang="ts">
	import { Download, Grid2x2, Maximize, Moon, Sun } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import ZoomControl from './ZoomControl.svelte'
	import { useZoomPan } from './zoomPan.svelte'
	import { twMerge } from 'tailwind-merge'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'

	interface Props {
		src: string
		/** Shown in the header and used as the download's file name. */
		name: string
		/** Byte size, shown beside the dimensions when known. */
		bytes?: number
	}

	let { src, name, bytes }: Props = $props()

	type Backdrop = 'checker' | 'light' | 'dark'
	const BACKDROPS = {
		light: { label: 'Light', icon: Sun },
		dark: { label: 'Dark', icon: Moon },
		checker: { label: 'Checkerboard', icon: Grid2x2 }
	} as const

	// Room kept around a fitted image so its edges stay visible against the backdrop.
	const view = useZoomPan({ margin: { x: 32, y: 32 } })
	// The source that failed, not a flag: a new `src` (the file fixed and re-read) must mount
	// the image again, and only the image's own load event could clear a flag.
	let failedSrc = $state<string>()
	const failed = $derived(failedSrc === src)
	// Until the viewer picks one, the backdrop is Windmill's own theme, and follows it.
	const isDark = useIsDarkMode()
	let chosenBackdrop = $state<Backdrop | undefined>(undefined)
	const backdrop = $derived<Backdrop>(chosenBackdrop ?? (isDark.val ? 'dark' : 'light'))
	// The cycle starts at the theme's own backdrop, so the first click swaps light and dark.
	const backdropCycle = $derived<Backdrop[]>(
		isDark.val ? ['dark', 'light', 'checker'] : ['light', 'dark', 'checker']
	)
	const nextBackdrop = $derived(
		backdropCycle[(backdropCycle.indexOf(backdrop) + 1) % backdropCycle.length]
	)
	function humanSize(n: number): string {
		if (n < 1024) return `${n} B`
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
		return `${(n / (1024 * 1024)).toFixed(1)} MB`
	}

	// A 16px checkerboard, the usual stand-in for "transparent".
	const CHECKER_STYLE =
		'background-color:#ffffff;background-image:linear-gradient(45deg,#e5e7eb 25%,transparent 25%,transparent 75%,#e5e7eb 75%),linear-gradient(45deg,#e5e7eb 25%,transparent 25%,transparent 75%,#e5e7eb 75%);background-size:16px 16px;background-position:0 0,8px 8px'
</script>

<div class="flex flex-col h-full min-h-0 bg-surface">
	<!-- The file name is the tab's label already; this bar carries what the tab cannot. -->
	<div
		class="flex flex-wrap items-center justify-between gap-x-2 gap-y-1 px-4 py-2 border-b border-border-light"
	>
		<span class="text-xs text-secondary whitespace-nowrap tabular-nums" title={name}>
			{#if view.natural}
				{view.natural.w} × {view.natural.h}{bytes !== undefined ? ` · ${humanSize(bytes)}` : ''}
			{/if}
		</span>
		<div class="flex flex-wrap items-center gap-1">
			<ZoomControl
				scale={view.natural ? view.scale : undefined}
				min={view.minScale}
				max={view.maxScale}
				onZoomOut={view.zoomOut}
				onZoomIn={view.zoomIn}
				onZoomTo={view.zoomToAnimated}
			/>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: Maximize }}
				title="Fit to panel (0)"
				disabled={!view.natural}
				onClick={view.fit}
			/>
			<Button
				unifiedSize="sm"
				variant={view.zoom === 1 ? 'default' : 'subtle'}
				title="Actual size (1)"
				disabled={!view.natural}
				onClick={view.actualSize}
			>
				1:1
			</Button>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: BACKDROPS[backdrop].icon }}
				title={`${BACKDROPS[backdrop].label} background, click for ${BACKDROPS[nextBackdrop].label.toLowerCase()}`}
				onClick={() => (chosenBackdrop = nextBackdrop)}
			/>
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: Download }}
				title="Download"
				href={src}
				download={name}
			/>
		</div>
	</div>

	<!-- The viewport takes keys and the wheel itself: zoom follows the cursor, so it has to
	     be the element under it. ARIA's `application` role is the one for a surface that
	     handles its own pointer and keys, but Svelte does not count it as interactive. -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
	<div
		class={twMerge(
			'relative flex-1 min-h-0 overflow-hidden outline-none select-none touch-none',
			backdrop === 'light' && 'bg-white',
			// The dark theme's surface-secondary in either theme: the light theme holds it as `-inverse`.
			backdrop === 'dark' && (isDark.val ? 'bg-surface-secondary' : 'bg-surface-secondary-inverse'),
			view.pannable ? (view.panning ? 'cursor-grabbing' : 'cursor-grab') : 'cursor-default'
		)}
		style={backdrop === 'checker' ? CHECKER_STYLE : undefined}
		{@attach view.surface}
		tabindex="0"
		role="application"
		aria-roledescription="image viewer"
		aria-label={`${name}. Plus and minus zoom, 0 fits, 1 shows actual size.`}
		onkeydown={(e) => {
			if (view.handleKey(e)) e.preventDefault()
		}}
	>
		{#if failed}
			<div class="absolute inset-0 flex items-center justify-center p-4">
				<span class="rounded-md bg-surface px-3 py-2 text-xs text-secondary shadow-sm">
					This file could not be displayed as an image.
				</span>
			</div>
		{:else}
			<img
				{@attach view.image}
				{src}
				alt={name}
				draggable="false"
				onerror={() => (failedSrc = src)}
				class={view.imageClass}
				style={view.imageStyle}
			/>
		{/if}
	</div>
</div>
