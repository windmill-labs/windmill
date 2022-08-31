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
			firstHeight: top.offsetHeight,
			secondHeight: down.offsetHeight
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
		delta.y = Math.min(Math.max(delta.y, -md.firstHeight), md.secondHeight)

		separator.style.top = md.offsetTop + delta.y + 'px'
		top.style.height = md.firstHeight + delta.y + 'px'
		down.style.height = md.secondHeight - delta.y + 'px'
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
		if (top) top.removeAttribute('style')
		if (down) down.removeAttribute('style')
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
	let top, down
	export let topPanelSize = '50%'
	export let downPanelSize = '50%'
	export let minTopPaneSize = '0'
	export let minDownPaneSize = '0'
	$: topPanelSize && resetSize()
	$: downPanelSize && resetSize()
</script>

<div
	class="wrapper"
	style="--top-panel-size: {topPanelSize}; --down-panel-size: {downPanelSize}; --min-top-panel-size:{minTopPaneSize}; --min-down-panel-size: {minDownPaneSize};"
>
	<div bind:this={top} class="top">
		<slot name="top">
			<div style="background-color: red;">Top Contents goes here...</div>
		</slot>
	</div>
	<div
		bind:this={separator}
		class="separator"
		on:mousedown={onMouseDownWrapper}
		on:touchstart={onMouseDownWrapper}
	/>
	<div bind:this={down} class="down">
		<slot name="down">
			<div style="background-color: yellow;">Down Contents goes here...</div>
		</slot>
	</div>
</div>

<style>
	div.wrapper {
		width: 100%;
		height: 100%;
		/* background-color: yellow; */
		display: flex;
		flex-direction: column;
	}
	div.separator {
		cursor: row-resize;
		width: 100%;
		height: 1px;
		z-index: 1;

		@apply bg-gray-400;
	}

	div.separator:hover {
		box-shadow: 0px 0px 5px 0px black;
	}

	div.top {
		height: var(--top-panel-size);
		min-height: var(--min-top-panel-size);
		width: 100%;
	}
	div.down {
		height: var(--down-panel-size);
		min-height: var(--min-down-panel-size);
		width: 100%;
	}
</style>
