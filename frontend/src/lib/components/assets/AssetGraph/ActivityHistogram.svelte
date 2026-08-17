<script lang="ts">
	import { isActiveEvent, type PipelineEvent } from './activeRunnables.svelte'

	interface Props {
		// All runs in the loaded window — bucketed into the bars. Filtering is
		// the caller's job; this always shows the whole window so you can brush
		// within it.
		events: PipelineEvent[]
		// Window bounds in ms (start … end). Bars span this range.
		from: number
		to: number
		// Current brushed selection in ms, or undefined for the whole window.
		selected?: { from: number; to: number } | undefined
		// Emitted on brush end; `undefined` for a click (clears the selection).
		onSelect: (range: { from: number; to: number } | undefined) => void
	}
	let { events, from, to, selected, onSelect }: Props = $props()

	const BUCKETS = 48

	let bins = $derived.by(() => {
		const span = Math.max(1, to - from)
		const arr = Array.from({ length: BUCKETS }, () => ({ success: 0, failure: 0 }))
		for (const e of events) {
			// Future-scheduled queued runs aren't activity.
			if (e.status === 'queued' && !isActiveEvent(e)) continue
			const t = Date.parse(e.at)
			if (isNaN(t) || t < from || t > to) continue
			const i = Math.min(BUCKETS - 1, Math.max(0, Math.floor(((t - from) / span) * BUCKETS)))
			if (e.status === 'failure') arr[i].failure += 1
			else arr[i].success += 1 // success + running/active
		}
		return arr
	})
	let maxCount = $derived(Math.max(1, ...bins.map((b) => b.success + b.failure)))

	// Selection rendered as a fractional band [0,1] over the bar area.
	let selFrac = $derived.by(() => {
		if (!selected) return undefined
		const span = Math.max(1, to - from)
		const a = Math.max(0, Math.min(1, (selected.from - from) / span))
		const b = Math.max(0, Math.min(1, (selected.to - from) / span))
		return a < b ? { left: a, width: b - a } : undefined
	})

	let barsEl: HTMLDivElement | undefined = $state()
	let dragFrom = $state<number | undefined>(undefined) // fractional x at pointerdown
	let dragTo = $state<number | undefined>(undefined)

	function fracAt(clientX: number): number {
		if (!barsEl) return 0
		const r = barsEl.getBoundingClientRect()
		return Math.max(0, Math.min(1, (clientX - r.left) / Math.max(1, r.width)))
	}
	function fracToMs(f: number): number {
		return from + f * Math.max(1, to - from)
	}
	function onPointerDown(e: PointerEvent) {
		barsEl?.setPointerCapture(e.pointerId)
		dragFrom = fracAt(e.clientX)
		dragTo = dragFrom
	}
	function onPointerMove(e: PointerEvent) {
		if (dragFrom == undefined) return
		dragTo = fracAt(e.clientX)
	}
	function onPointerUp(e: PointerEvent) {
		if (dragFrom == undefined) return
		const a = dragFrom
		const b = fracAt(e.clientX)
		dragFrom = undefined
		dragTo = undefined
		// A drag of any meaningful width selects; a click (tiny delta) clears.
		if (Math.abs(b - a) < 0.012) {
			onSelect(undefined)
			return
		}
		const lo = Math.min(a, b)
		const hi = Math.max(a, b)
		onSelect({ from: Math.floor(fracToMs(lo)), to: Math.ceil(fracToMs(hi)) })
	}

	// Live drag band (while dragging) or the committed selection.
	let band = $derived.by(() => {
		if (dragFrom != undefined && dragTo != undefined) {
			const lo = Math.min(dragFrom, dragTo)
			const hi = Math.max(dragFrom, dragTo)
			return { left: lo, width: hi - lo }
		}
		return selFrac
	})

	function bucketRange(i: number): { from: number; to: number } {
		const span = Math.max(1, to - from)
		return { from: from + (i / BUCKETS) * span, to: from + ((i + 1) / BUCKETS) * span }
	}
	function fmtTime(ms: number): string {
		return new Date(ms).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		})
	}
</script>

<div class="px-2 pt-1.5 pb-1 select-none">
	<div
		bind:this={barsEl}
		class="relative flex items-end gap-px h-9 cursor-crosshair"
		role="presentation"
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		title="Drag to filter to a time range; click to clear"
	>
		{#each bins as b, i (i)}
			{@const total = b.success + b.failure}
			{@const br = bucketRange(i)}
			<div
				class="flex-1 min-w-0 flex flex-col justify-end h-full"
				style="opacity: {total ? 1 : 0.35}"
				title={`${fmtTime(br.from)} – ${fmtTime(br.to)}${total ? ` · ${b.success} ok${b.failure ? `, ${b.failure} failed` : ''}` : ' · no runs'}`}
			>
				{#if b.failure > 0}
					<div
						class="w-full bg-red-400 dark:bg-red-500/80 rounded-t-[1px]"
						style="height: {(b.failure / maxCount) * 100}%"
					></div>
				{/if}
				{#if b.success > 0}
					<div
						class="w-full bg-emerald-400 dark:bg-emerald-500/80"
						class:rounded-t-[1px]={b.failure === 0}
						style="height: {(b.success / maxCount) * 100}%"
					></div>
				{/if}
				{#if total === 0}
					<div class="w-full bg-surface-secondary" style="height: 2px"></div>
				{/if}
			</div>
		{/each}
		{#if band}
			<div
				class="absolute top-0 bottom-0 bg-blue-500/15 border-x border-blue-500/50 pointer-events-none"
				style="left: {band.left * 100}%; width: {band.width * 100}%"
			></div>
		{/if}
	</div>
	<!-- Always-on window axis: the period the bars span. The `from` prefix keeps
	     the left edge from reading as a date header for the run list below it. -->
	<div class="flex justify-between text-3xs text-tertiary tabular-nums mt-0.5">
		<span>from {fmtTime(from)}</span>
		<span>now</span>
	</div>
</div>
