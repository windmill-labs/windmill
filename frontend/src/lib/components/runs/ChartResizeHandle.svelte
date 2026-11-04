<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		height: number
		min: number
		max: number
		defaultHeight: number
		class?: string
	}

	let {
		height = $bindable(),
		min,
		max,
		defaultHeight,
		class: className = ''
	}: Props = $props()

	const KEYBOARD_STEP_PX = 16

	let drag: { startY: number; startHeight: number } | undefined = $state(undefined)

	function clamp(h: number) {
		return Math.round(Math.min(Math.max(h, min), Math.max(min, max)))
	}

	function onpointerdown(e: PointerEvent & { currentTarget: HTMLDivElement }) {
		if (e.button !== 0) return
		drag = { startY: e.clientY, startHeight: height }
		e.currentTarget.setPointerCapture(e.pointerId)
		e.preventDefault()
	}

	function onpointermove(e: PointerEvent) {
		if (drag) height = clamp(drag.startHeight + e.clientY - drag.startY)
	}

	function endDrag() {
		drag = undefined
	}

	function ondblclick() {
		height = defaultHeight
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return
		e.preventDefault()
		// The runs table reads these keys at `window` to move its selected run.
		e.stopPropagation()
		height = clamp(height + (e.key === 'ArrowDown' ? KEYBOARD_STEP_PX : -KEYBOARD_STEP_PX))
	}
</script>

<!-- A focusable separator is the WAI-ARIA window splitter, which svelte counts as non-interactive. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	role="separator"
	aria-orientation="horizontal"
	aria-label="Resize chart"
	aria-valuemin={min}
	aria-valuemax={Math.max(min, max)}
	aria-valuenow={height}
	tabindex="0"
	title="Drag to resize, double-click to reset"
	class={twMerge(
		'group flex h-3 cursor-row-resize touch-none items-center justify-center focus-visible:outline-none',
		className
	)}
	{onpointerdown}
	{onpointermove}
	onpointerup={endDrag}
	onpointercancel={endDrag}
	{ondblclick}
	{onkeydown}
>
	<div
		class={twMerge(
			'h-1 w-9 rounded-full bg-border-normal transition-colors group-hover:bg-surface-accent-primary group-focus-visible:bg-surface-accent-primary',
			drag ? 'bg-surface-accent-primary' : ''
		)}
	></div>
</div>
