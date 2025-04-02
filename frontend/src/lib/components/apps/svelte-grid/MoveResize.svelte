<script lang="ts">
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import {
		computePosition,
		findGridItemParentGrid,
		isContainer,
		type GridShadow
	} from '../editor/appUtils'
	import { throttle } from './utils/other'

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
	export let overlapped: string | undefined = undefined
	export let moveMode: 'move' | 'insert' = 'move'
	export let type: string | undefined = undefined
	export let fakeShadow: GridShadow | undefined = undefined
	export let disableMove: boolean = true
	export let mounted: boolean = false

	const ctx = getContext<AppEditorContext>('AppEditorContext')
	const { mode, app } = getContext<AppViewerContext>('AppViewerContext')

	const scale = ctx ? ctx.scale : writable(100)

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

	onMount(() => {
		if (ctx && $mode == 'dnd') {
			ctx.dndItem.update((x) => {
				x[id] = (moveX, moveY, topY) => {
					ctx.componentActive.set(true)
					let gridItem = document.getElementById(divId)
					let irect = gridItem?.getBoundingClientRect()

					const clientX = (irect?.x ?? 0) + (irect?.width ?? 0)
					const clientY = irect?.y ?? 0
					initX = (clientX / $scale) * 100
					initY = (clientY / $scale) * 100

					window.addEventListener('pointermove', pointermove)
					window.addEventListener('pointerup', pointerup)

					dispatch('initmove')

					const cordDiff = {
						x: (moveX / $scale) * 100 - initX,
						y: (moveY / $scale) * 100 - initY
					}

					dispatch('move', { cordDiff, clientY: clientY })
				}
				return x
			})
		}
	})

	export function inActivate() {
		if (shadowElement && shadow != undefined) {
			let subgrid = shadowElement.closest('.subgrid')
			let irect = shadowElement.getBoundingClientRect()
			let shadowBound

			const factor = $scale / 100

			if (subgrid) {
				let prect = subgrid.getBoundingClientRect()

				shadowBound = {
					x: irect.x - prect.left,
					y: irect.y - prect.top
				}
			} else {
				shadowBound = irect
			}

			const xdragBound = rect.left + cordDiff.x
			const ydragBound = rect.top + cordDiff.y

			cordDiff.x = shadow.x * xPerPx + gapX - (shadowBound.x - xdragBound * factor) * factor
			cordDiff.y = shadow.y * yPerPx + gapY - (shadowBound.y - ydragBound * factor) * factor

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
			let prect = subgrid.getBoundingClientRect()

			const factor = $scale / 100

			rect = {
				top: (irect.top - prect.top) / factor,
				left: (irect.left - prect.left) / factor
			}
		} else {
			rect = irect
		}
	}

	let dragClosure: (() => void) | undefined = undefined
	const pointerdown = ({ clientX, clientY }) => {
		dragClosure = () => {
			dragClosure = undefined
			ctx.componentActive.set(true)

			initX = (clientX / $scale) * 100
			initY = (clientY / $scale) * 100
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
	let intervalId: NodeJS.Timeout | undefined = undefined

	const stopAutoscroll = () => {
		intervalId && clearInterval(intervalId)
		intervalId = undefined
		sign = { x: 0, y: 0 }
		vel = { x: 0, y: 0 }
	}

	const update = () => {
		if (xPerPx != 0) {
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
	}

	// Shared state for the shadow position
	let currentShadowPosition:
		| { x: number; y: number; xPerPx: number; yPerPx: number; h: number; w: number }
		| undefined = undefined
	let currentIntersectingElementId: string | undefined = undefined

	let moving: boolean = false

	const pointermove = (event) => {
		dragClosure && dragClosure()
		event.preventDefault()
		event.stopPropagation()
		event.stopImmediatePropagation()

		moving = true

		const { clientX, clientY } = event
		const cordDiff = { x: (clientX / $scale) * 100 - initX, y: (clientY / $scale) * 100 - initY }

		if (moveMode === 'move') {
			dispatch('move', {
				cordDiff,
				clientY,
				intersectingElement: undefined,
				shadow: undefined
			})

			return
		}

		throttledComputeShadow(clientX, clientY)

		dispatch('move', {
			cordDiff,
			clientY,
			intersectingElement: currentIntersectingElementId,
			shadow: currentShadowPosition,
			overlapped
		})
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

	let element: HTMLElement | undefined = undefined

	function computeShadow(clientX: number, clientY: number) {
		const elementsAtPoint = document.elementsFromPoint(clientX, clientY)
		const intersectingElement = elementsAtPoint.find(
			(el) =>
				el.id !== divId &&
				el.classList.contains('svlt-grid-item') &&
				el.getAttribute('data-iscontainer') === 'true'
		)

		const newOverlapped = intersectingElement ? intersectingElement?.id.split('-')[1] : undefined

		const container = newOverlapped
			? intersectingElement?.querySelector('.svlt-grid-container')
			: document.getElementById('root-grid')

		const xPerPxComputed = Number(container?.getAttribute('data-xperpx')) ?? xPerPx

		const position = computePosition(
			clientX,
			clientY,
			xPerPxComputed,
			yPerPx,
			newOverlapped,
			element
		)

		if (overlapped !== newOverlapped) {
			currentShadowPosition = undefined
		} else {
			// Update shared state
			currentShadowPosition = {
				x: position.x,
				y: position.y,
				xPerPx: xPerPxComputed,
				yPerPx,
				h: item.h,
				w: item.w
			}
		}

		currentIntersectingElementId = newOverlapped
	}

	const throttledComputeShadow = throttle((clientX, clientY) => {
		computeShadow(clientX, clientY)
	}, 50)

	const pointerup = (e) => {
		ctx.componentActive.set(false)
		stopAutoscroll()

		window.removeEventListener('pointerdown', pointerdown)
		window.removeEventListener('pointermove', pointermove)
		window.removeEventListener('pointerup', pointerup)

		if (!dragClosure) {
			repaint(true, true)
		} else {
			dragClosure = undefined
		}

		if (!moving) {
			return
		}

		const parent = findGridItemParentGrid($app, id)

		if (overlapped && (overlapped === parent || parent?.startsWith(overlapped))) {
			return
		}

		dispatch('dropped', {
			id,
			overlapped,
			x: fakeShadow?.x,
			y: fakeShadow?.y
		})
	}

	let resizeInitPos = { x: 0, y: 0 }
	let initSize = { width: 0, height: 0 }

	let orientation: 'both' | 'horizontal' | 'vertical' = 'both'
	const resizePointerDown = (e, o: 'both' | 'horizontal' | 'vertical') => {
		e.stopPropagation()
		orientation = o
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
		dispatch('resizeStart')
	}

	const resizePointerMove = ({ pageX, pageY }) => {
		if (shadow) {
			newSize.width =
				orientation == 'both' || orientation == 'horizontal'
					? initSize.width + ((pageX - resizeInitPos.x) / $scale) * 100
					: initSize.width
			newSize.height =
				orientation == 'both' || orientation == 'vertical'
					? initSize.height + ((pageY - resizeInitPos.y) / $scale) * 100
					: initSize.height

			// Get max col number
			let maxWidth = cols - shadow.x
			maxWidth = maxWidth

			// Limit bound
			newSize.width = Math.min(newSize.width, maxWidth * xPerPx - gapX * 2)

			if (xPerPx) {
				// Limit col & row
				shadow.w = Math.round(Math.max((newSize.width + gapX * 2) / xPerPx, 1))
				shadow.h = Math.round(Math.max((newSize.height + gapY * 2) / yPerPx, 1))
			}
			repaint(false, false)
		}
	}

	const resizePointerUp = (e) => {
		e.stopPropagation()
		repaint(true, true)

		window.removeEventListener('pointermove', resizePointerMove)
		window.removeEventListener('pointerup', resizePointerUp)
		dispatch('resizeEnd')
	}

	function shouldDisplayShadow(moveMode: 'insert' | 'move', overlapped: string | undefined) {
		if (moveMode === 'move') {
			return true
		}

		const parent = findGridItemParentGrid($app, id)

		if (parent === undefined) {
			return overlapped === undefined
		} else if (overlapped) {
			return parent.startsWith(overlapped)
		}

		return false
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	bind:this={element}
	draggable="false"
	on:pointerdown|stopPropagation|preventDefault={pointerdown}
	id={divId}
	class="svlt-grid-item"
	data-iscontainer={type ? isContainer(type) : false}
	data-componenttype={type}
	class:svlt-grid-active={active || (trans && rect)}
	style="width: {xPerPx == 0 ? 0 : active ? newSize.width : width}px; height:{xPerPx == 0
		? 0
		: active
		? newSize.height
		: height}px; 
	{xPerPx == 0 ? 'overflow: hidden;' : ''}
	{onTop ? 'z-index: 1000;' : ''}
	
  {active && rect
		? `transform: translate(${cordDiff.x}px, ${cordDiff.y}px);top:${rect.top}px;left:${rect.left}px;z-index:10000;`
		: trans
		? `transform: translate(${cordDiff.x}px, ${cordDiff.y}px); position:absolute; transition: width 0.2s, height 0.2s;`
		: `${
				xPerPx > 0 && mounted ? 'transition: transform 0.1s, opacity 0.1s;' : ''
		  } transform: translate(${left}px, ${top}px); `} "
>
	<slot />
	{#if moveMode === 'move' && !disableMove}
		<div
			class="svlt-grid-resizer-bottom"
			on:pointerdown={(e) => resizePointerDown(e, 'vertical')}
		></div>
		<div
			class="svlt-grid-resizer-side"
			on:pointerdown={(e) => resizePointerDown(e, 'horizontal')}
		></div>
		<div class="svlt-grid-resizer" on:pointerdown={(e) => resizePointerDown(e, 'both')}></div>
	{/if}
</div>

{#if xPerPx > 0 && (active || trans) && shadow}
	<div
		class={twMerge(
			'svlt-grid-shadow shadow-active',
			shouldDisplayShadow(moveMode, overlapped) ? '' : 'hidden'
		)}
		style="width: {shadow.w * xPerPx - gapX * 2}px; height: {shadow.h * yPerPx -
			gapY * 2}px; transform: translate({shadow.x * xPerPx + gapX}px, {shadow.y * yPerPx +
			gapY}px); "
		bind:this={shadowElement}
	></div>
{/if}

<style>
	.svlt-grid-item {
		touch-action: none;
		position: absolute;
		will-change: auto;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}

	.svlt-grid-resizer-bottom {
		user-select: none;
		width: 100%;
		height: 4px;
		position: absolute;
		bottom: 0;
		cursor: s-resize;
	}

	.svlt-grid-resizer-side {
		user-select: none;
		width: 4px;
		height: 100%;
		position: absolute;
		top: 0;
		right: 0;
		cursor: e-resize;
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
		z-index: 300;
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
		will-change: transform;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
		background: #93c4fdd0;
	}
</style>
