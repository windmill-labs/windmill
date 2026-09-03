<script lang="ts">
	/**
	 * The reasoning-effort control: a thin slider over a model's ordered effort stops.
	 *
	 * Presentational on purpose. Callers keep their own value convention — the copilot's
	 * REASONING_OFF sentinel and an agent's `reasoning_effort` token mean off in
	 * different ways — and hand this component a resolved list of stops plus the current
	 * one, so the two never have to agree on anything but the ordering.
	 */
	interface Props {
		/** Ordered stops, least effort first. Fewer than two renders no slider. */
		stops: string[]
		current: string
		onSelect: (stop: string) => void
		label?: string
		/** When set, the section renders disabled with this as the explanation. */
		unsupportedReason?: string
		disabled?: boolean
		/** Display name for a stop whose value is a provider sentinel rather than a word. */
		format?: (stop: string) => string
		/** Shown in place of the current stop — a state the slider has no position for. */
		overrideLabel?: string
	}

	let {
		stops,
		current,
		onSelect,
		label = 'Thinking',
		unsupportedReason,
		disabled = false,
		format = (stop: string) => stop,
		overrideLabel
	}: Props = $props()

	const stopIndex = $derived(Math.max(0, stops.indexOf(current)))
	// Percentage filled (accent) up to the thumb; the rest of the track stays surface-secondary.
	const fillPct = $derived(
		stops.length > 1 ? Math.round((stopIndex / (stops.length - 1)) * 100) : 0
	)

	/** Left/right stepping, for a caller that owns the keyboard (a melt menu item). */
	export function adjust(e: KeyboardEvent) {
		if (disabled || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return
		e.preventDefault()
		const next = Math.min(
			stops.length - 1,
			Math.max(0, stopIndex + (e.key === 'ArrowRight' ? 1 : -1))
		)
		onSelect(stops[next])
	}

	// Melt's roving focus blurs the focused element on pointermove, which aborts a native
	// thumb drag. Direct (non-delegated) listeners so they run before melt's item listener.
	function isolatePointer(node: HTMLElement) {
		const stop = (e: Event) => e.stopPropagation()
		node.addEventListener('pointerdown', stop)
		node.addEventListener('pointermove', stop)
		return {
			destroy() {
				node.removeEventListener('pointerdown', stop)
				node.removeEventListener('pointermove', stop)
			}
		}
	}
</script>

{#if unsupportedReason}
	<!-- Kept visible rather than hidden: the absence of the control is itself the answer,
	     but only if it says why. -->
	<div class="px-3 pt-1 pb-1.5 opacity-60 cursor-default" aria-disabled="true">
		<div class="text-2xs uppercase tracking-wide text-secondary">{label}</div>
		<div class="text-2xs text-tertiary mt-0.5">{unsupportedReason}</div>
	</div>
{:else}
	<div class="px-3 pt-1 pb-0.5 flex items-center justify-between" class:opacity-60={disabled}>
		<span class="text-2xs uppercase tracking-wide text-secondary">{label}</span>
		<span class="text-2xs text-secondary tabular-nums">{overrideLabel ?? format(current)}</span>
	</div>
	{#if stops.length > 1}
		<!-- Only the slider area reflects an enclosing menu item's highlight, not the header. -->
		<div class="px-3 py-1.5 rounded-sm transition-colors group-data-[highlighted]:bg-surface-hover">
			<input
				type="range"
				min="0"
				max={stops.length - 1}
				step="1"
				value={stopIndex}
				style="--fill: {fillPct}%"
				{disabled}
				oninput={(e) => onSelect(stops[+e.currentTarget.value])}
				use:isolatePointer
				class="lean-range no-default-style w-full"
				aria-label="Reasoning effort"
			/>
		</div>
	{/if}
{/if}

<style>
	/* Lean reasoning slider: a thin track and a small, borderless accent thumb. Native range
	   thumbs can't be styled with Tailwind, and Svelte prunes scoped vendor pseudo-element
	   rules — so they are wrapped in :global (the class is unique to this component). */
	.lean-range {
		-webkit-appearance: none;
		appearance: none;
		height: 10px;
		margin: 0;
		padding: 0;
		/* override the global `input { background-color: ... !important }` so only the
		   thin track shows, not a full-height band behind it */
		background-color: transparent !important;
		cursor: pointer;
		outline: none;
	}
	.lean-range:focus,
	.lean-range:focus-visible {
		outline: none;
	}
	:global(.lean-range::-webkit-slider-runnable-track) {
		height: 3px;
		border-radius: 9999px;
		background: linear-gradient(
			to right,
			rgb(var(--color-surface-accent-primary)) var(--fill, 0%),
			rgb(var(--color-surface-secondary)) var(--fill, 0%)
		);
	}
	:global(.lean-range::-webkit-slider-thumb) {
		-webkit-appearance: none;
		appearance: none;
		margin-top: -3.5px;
		width: 10px;
		height: 10px;
		border: none;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
	:global(.lean-range::-moz-range-track) {
		height: 3px;
		border-radius: 9999px;
		background: rgb(var(--color-surface-secondary));
	}
	:global(.lean-range::-moz-range-progress) {
		height: 3px;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
	:global(.lean-range::-moz-range-thumb) {
		width: 10px;
		height: 10px;
		border: none;
		border-radius: 9999px;
		background: rgb(var(--color-surface-accent-primary));
	}
</style>
