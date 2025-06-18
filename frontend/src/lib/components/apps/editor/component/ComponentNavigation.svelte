<script lang="ts">
	import { getContext } from 'svelte'
	import {
		handleEscape,
		handlePaste,
		handleArrowUp,
		handleCut,
		handleCopy,
		down,
		right,
		left
	} from './componentCallbacks.svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'

	const { history, movingcomponents, jobsDrawerOpen } = getContext<AppEditorContext>(
		'AppEditorContext'
	) as AppEditorContext
	const { app, selectedComponent, focusedGrid, componentControl } = getContext<AppViewerContext>(
		'AppViewerContext'
	) as AppViewerContext
	const ctx = {
		history,
		app,
		selectedComponent,
		focusedGrid,
		componentControl,
		movingcomponents,
		jobsDrawerOpen
	}
	function keydown(event: KeyboardEvent) {
		// console.log('keydown', $focusedGrid, $selectedComponent)
		// Ignore keydown events if the user is typing in monaco
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}
		switch (event.key) {
			case 'Escape':
				handleEscape(event, ctx)
				break

			case 'ArrowUp': {
				handleArrowUp(event, ctx)
				break
			}

			case 'ArrowDown': {
				down(event, ctx)
				break
			}

			case 'ArrowRight': {
				right(event, ctx)
				break
			}

			case 'ArrowLeft': {
				left(event, ctx)
				break
			}

			case 'c':
				if (event.ctrlKey || event.metaKey) {
					handleCopy(event, ctx)
				}
				break

			case 'x':
				if (event.ctrlKey || event.metaKey) {
					handleCut(event, ctx)
				}
				break

			default:
				break
		}
	}
</script>

<svelte:window onkeydown={keydown} onpaste={(e) => handlePaste(e, ctx)} />
