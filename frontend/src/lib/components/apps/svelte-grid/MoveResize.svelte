<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let sensor
	export let width
	export let height
	export let left
	export let top

	export let id
	export let container

	export let xPerPx
	export let yPerPx

	export let gapX
	export let gapY
	export let item

	export let cols

	export let nativeContainer
	export let onTop
	export let shadow: { x: number; y: number; w: number; h: number } | undefined = undefined

	const divId = `component-${id}`
	let shadowElement

	let active = false

	let initX, initY

	let capturePos = {
		x: 0,
		y: 0
	}

	let cordDiff = { x: 0, y: 0 }

	let newSize = { width, height }
	let trans = false

	let anima

	export function inActivate() {
		if (shadowElement && shadow != undefined) {
			let subgrid = shadowElement.closest('.subgrid')
			let irect = shadowElement.getBoundingClientRect()
			let shadowBound

			if (subgrid) {
				let subGridParent = subgrid.parentElement
				let subGridParentRect = subGridParent.getBoundingClientRect()
				let prect = subgrid.getBoundingClientRect()

				const subGridOffset = {
					x: subGridParentRect.x - prect.x,
					y: subGridParentRect.y - prect.y
				}

				shadowBound = {
					x: irect.x - prect.left - subGridOffset.x,
					y: irect.y - prect.top - subGridOffset.y
				}
			} else {
				shadowBound = irect
			}

			const xdragBound = rect.left + cordDiff.x
			const ydragBound = rect.top + cordDiff.y

			cordDiff.x = shadow.x * xPerPx + gapX - (shadowBound.x - xdragBound)
			cordDiff.y = shadow.y * yPerPx + gapY - (shadowBound.y - ydragBound)

			active = false
			trans = true

			clearTimeout(anima)

			anima = setTimeout(() => {
				trans = false
			}, 50)
		}
	}

	let repaint = (activate: boolean, isPointerUp: boolean) => {
		dispatch('repaint', {
			id,
			isPointerUp,
			activate
		})
	}

	// Autoscroll
	let _scrollTop = 0
	let containerFrame
	let rect
	let scrollElement

	const getContainerFrame = (element) => {
		if (element === document.documentElement || !element) {
			const { top, bottom } = nativeContainer.getBoundingClientRect()

			return {
				top: Math.max(0, top),
				bottom: Math.min(window.innerHeight, bottom)
			}
		}

		return element.getBoundingClientRect()
	}

	const getScroller = (element) => (!element ? document.documentElement : element)

	function computeRect() {
		let gridItem = document.getElementById(divId)
		if (!gridItem) return
		let subgrid = gridItem.closest('.subgrid')

		let irect = gridItem.getBoundingClientRect()
		if (subgrid && subgrid.parentElement) {
			let subGridParent = subgrid.parentElement
			let subGridParentRect = subGridParent.getBoundingClientRect()

			let prect = subgrid.getBoundingClientRect()

			const subGridOffset = {
				x: subGridParentRect.x - prect.x,
				y: subGridParentRect.y - prect.y
			}

			rect = {
				top: irect.top - prect.top - subGridOffset.y,
				left: irect.left - prect.left - subGridOffset.x
			}
		} else {
			rect = irect
		}
	}

	let dragClosure: (() => void) | undefined = undefined
	const pointerdown = ({ clientX, clientY }) => {
		dragClosure = () => {
			dragClosure = undefined

			initX = clientX
			initY = clientY
			dispatch('initmove')
		}
		window.addEventListener('pointermove', pointermove)
		window.addEventListener('pointerup', pointerup)
	}

	export function initmove() {
		computeRect()
		newSize = { width, height }
		capturePos = { x: left, y: top }
		shadow = { x: item.x, y: item.y, w: item.w, h: item.h }
		cordDiff = { x: 0, y: 0 }
		active = true
		trans = false

		containerFrame = getContainerFrame(container)
		scrollElement = getScroller(container)

		_scrollTop = scrollElement.scrollTop
	}

	let sign = { x: 0, y: 0 }
	let vel = { x: 0, y: 0 }
	let intervalId: NodeJS.Timer | undefined = undefined

	const stopAutoscroll = () => {
		intervalId && clearInterval(intervalId)
		intervalId = undefined
		sign = { x: 0, y: 0 }
		vel = { x: 0, y: 0 }
	}

	const update = () => {
		const boundX = capturePos.x + cordDiff.x
		const _newScrollTop = (scrollElement?.scrollTop ?? 0) - (_scrollTop ?? 0)
		const boundY = capturePos.y + (cordDiff.y + _newScrollTop)

		let gridX = Math.round(boundX / xPerPx)
		let gridY = Math.round(boundY / yPerPx)

		if (shadow) {
			shadow.x = Math.max(Math.min(gridX, cols - shadow.w), 0)
			shadow.y = Math.max(gridY, 0)
		}
	}

	const pointermove = (event) => {
		dragClosure && dragClosure()
		event.preventDefault()
		event.stopPropagation()
		event.stopImmediatePropagation()

		const { clientX, clientY } = event
		const cordDiff = { x: clientX - initX, y: clientY - initY }

		dispatch('move', { cordDiff, clientY })
	}

	export function updateMove(newCoordDiff, clientY) {
		if (!active) {
			active = true
		}
		if (trans) {
			trans = false
		}
		cordDiff = newCoordDiff
		// console.log(cordDiff, id, 'B')
		const Y_SENSOR = sensor

		if (containerFrame) {
			let velocityTop = Math.max(0, (containerFrame.top + Y_SENSOR - clientY) / Y_SENSOR)
			let velocityBottom = Math.max(0, (clientY - (containerFrame.bottom - Y_SENSOR)) / Y_SENSOR)

			const topSensor = velocityTop > 0 && velocityBottom === 0
			const bottomSensor = velocityBottom > 0 && velocityTop === 0

			sign.y = topSensor ? -1 : bottomSensor ? 1 : 0
			vel.y = sign.y === -1 ? velocityTop : velocityBottom

			if (vel.y > 0) {
				if (!intervalId) {
					// Start scrolling
					// TODO Use requestAnimationFrame
					intervalId = setInterval(() => {
						scrollElement.scrollTop += 2 * (vel.y + Math.sign(vel.y)) * sign.y
						update()
					}, 10)
				}
			} else if (intervalId) {
				stopAutoscroll()
			} else {
				update()
			}
		} else {
			update()
		}
	}
	const pointerup = (e) => {
		stopAutoscroll()

		window.removeEventListener('pointerdown', pointerdown)
		window.removeEventListener('pointermove', pointermove)
		window.removeEventListener('pointerup', pointerup)
		if (!dragClosure) {
			repaint(true, true)
		} else {
			dragClosure = undefined
		}
	}

	let resizeInitPos = { x: 0, y: 0 }
	let initSize = { width: 0, height: 0 }

	const resizePointerDown = (e) => {
		e.stopPropagation()
		const { pageX, pageY } = e

		resizeInitPos = { x: pageX, y: pageY }
		initSize = { width, height }

		cordDiff = { x: 0, y: 0 }

		computeRect()

		newSize = { width, height }

		active = true
		trans = false
		shadow = { x: item.x, y: item.y, w: item.w, h: item.h }

		containerFrame = getContainerFrame(container)
		scrollElement = getScroller(container)

		window.addEventListener('pointermove', resizePointerMove)
		window.addEventListener('pointerup', resizePointerUp)
	}

	const resizePointerMove = ({ pageX, pageY }) => {
		if (shadow) {
			newSize.width = initSize.width + pageX - resizeInitPos.x
			newSize.height = initSize.height + pageY - resizeInitPos.y

			// Get max col number
			let maxWidth = cols - shadow.x
			maxWidth = maxWidth

			// Limit bound
			newSize.width = Math.min(newSize.width, maxWidth * xPerPx - gapX * 2)

			// Limit col & row
			shadow.w = Math.round((newSize.width + gapX * 2) / xPerPx)
			shadow.h = Math.round((newSize.height + gapY * 2) / yPerPx)

			repaint(false, false)
		}
	}

	const resizePointerUp = (e) => {
		e.stopPropagation()
		repaint(true, true)

		window.removeEventListener('pointermove', resizePointerMove)
		window.removeEventListener('pointerup', resizePointerUp)
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	draggable="false"
	on:pointerdown|stopPropagation|preventDefault={pointerdown}
	id={divId}
	class="svlt-grid-item"
	class:svlt-grid-active={active || (trans && rect)}
	style="width: {active ? newSize.width : width}px; height:{active
		? newSize.height
		: height}px; {onTop ? 'z-index: 1000;' : ''}
  {active && rect
		? `transform: translate(${cordDiff.x}px, ${cordDiff.y}px);top:${rect.top}px;left:${rect.left}px;`
		: trans
		? `transform: translate(${cordDiff.x}px, ${cordDiff.y}px); position:absolute; transition: width 0.2s, height 0.2s;`
		: `transition: transform 0.1s, opacity 0.1s; transform: translate(${left}px, ${top}px); `} "
>
	<slot />
	<div class="svlt-grid-resizer" on:pointerdown={resizePointerDown} />
</div>

{#if (active || trans) && shadow}
	<div
		class="svlt-grid-shadow shadow-active"
		style="width: {shadow.w * xPerPx - gapX * 2}px; height: {shadow.h * yPerPx -
			gapY * 2}px; transform: translate({shadow.x * xPerPx + gapX}px, {shadow.y * yPerPx +
			gapY}px); "
		bind:this={shadowElement}
	/>
{/if}

<style>
	.svlt-grid-item {
		touch-action: none;
		position: absolute;
		will-change: auto;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}

	.svlt-grid-resizer {
		user-select: none;
		width: 20px;
		height: 20px;
		position: absolute;
		right: 0;
		bottom: 0;
		cursor: se-resize;
	}
	.svlt-grid-resizer::after {
		content: '';
		position: absolute;
		right: 3px;
		bottom: 3px;
		width: 5px;
		height: 5px;
		border-right: 2px solid rgba(0, 0, 0, 0.4);
		border-bottom: 2px solid rgba(0, 0, 0, 0.4);
	}

	.svlt-grid-active {
		z-index: 3;
		cursor: grabbing;
		position: fixed;
		opacity: 0.5;

		/*No user*/
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
		-moz-backface-visibility: hidden;
		-o-backface-visibility: hidden;
		-ms-backface-visibility: hidden;
		user-select: none;
	}

	.shadow-active {
		z-index: 2;
		transition: all 0.2s;
	}

	.svlt-grid-shadow {
		position: absolute;
		background: red;
		will-change: transform;
		background: pink;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}
</style>
