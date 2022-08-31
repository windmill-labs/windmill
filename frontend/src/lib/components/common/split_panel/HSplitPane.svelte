<script>
	import { onMount, onDestroy } from 'svelte'
	let separator
	export let updateCallback = () => {
		// do nothing
		return
	}
	export let onMouseDown = () => {
		// do nothing
		return
	}
	export let onMouseUp = () => {
		// do nothing
		return
	}
	let md
	const onMouseDownWrapper = (e) => {
		e.preventDefault()
		onMouseDown()
		if (e.button !== 0) return
		md = {
			e,
			offsetLeft: separator.offsetLeft,
			offsetTop: separator.offsetTop,
			firstWidth: left.offsetWidth,
			secondWidth: right.offsetWidth
		}
		window.addEventListener('mousemove', onMouseMove)
		window.addEventListener('mouseup', onMouseUpWrapper)
		window.addEventListener('touchmove', onMouseMove)
		window.addEventListener('touchend', onMouseUpWrapper)
	}
	const onMouseMove = (e) => {
		e.preventDefault()
		if (e.button !== 0) return
		var delta = { x: e.clientX - md.e.clientX, y: e.clientY - md.e.clientY }
		// Prevent negative-sized elements
		delta.x = Math.min(Math.max(delta.x, -md.firstWidth), md.secondWidth)
		separator.style.left = md.offsetLeft + delta.x + 'px'
		left.style.width = md.firstWidth + delta.x + 'px'
		right.style.width = md.secondWidth - delta.x + 'px'
		updateCallback()
	}
	const onMouseUpWrapper = (e) => {
		onMouseUp()
		if (e) {
			e.preventDefault()
			if (e.button !== 0) return
		}
		updateCallback()
		window.removeEventListener('mousemove', onMouseMove)
		window.removeEventListener('mouseup', onMouseUpWrapper)
		window.removeEventListener('touchmove', onMouseMove)
		window.removeEventListener('touchend', onMouseUpWrapper)
	}
	function resetSize() {
		if (left) left.removeAttribute('style')
		if (right) right.removeAttribute('style')
		if (separator) separator.removeAttribute('style')
	}
	function onResize() {
		onMouseUpWrapper()
		resetSize()
	}
	onMount(() => {
		window.addEventListener('resize', onResize)
	})
	onDestroy(() => {
		window.removeEventListener('resize', onResize)
	})

	let left, right
	export let leftPaneSize = '50%'
	export let rightPaneSize = '50%'
	export let minLeftPaneSize = '0'
	export let minRightPaneSize = '0'
	$: leftPaneSize && resetSize()
	$: rightPaneSize && resetSize()
</script>

<div
	class="w-full h-full inline-flex"
	style="--left-panel-size: {leftPaneSize}; --right-panel-size: {rightPaneSize}; --min-left-panel-size: {minLeftPaneSize}; --min-right-panel-size: {minRightPaneSize};"
>
	<div bind:this={left} class="left">
		<slot name="left">
			<div style="background-color: red;">Left Contents goes here...</div>
		</slot>
	</div>
	<div
		bind:this={separator}
		class="separator"
		on:mousedown={onMouseDownWrapper}
		on:touchstart={onMouseDownWrapper}
	/>
	<div bind:this={right} class="right">
		<slot name="right">
			<div style="background-color: yellow;">Right Contents goes here...</div>
		</slot>
	</div>
</div>

<style>
	div.separator {
		cursor: col-resize;
		height: 100%;
		width: 1px;
		margin-left: -2px;
		z-index: 1;

		@apply bg-gray-400;
	}

	div.separator:hover {
		box-shadow: 0px 0px 5px 0px black;
	}

	div.left {
		width: var(--left-panel-size);
		min-width: var(--min-left-panel-size);
		height: 100%;
	}
	div.right {
		width: var(--right-panel-size);
		min-width: var(--min-right-panel-size);
		height: 100%;
	}
</style>
