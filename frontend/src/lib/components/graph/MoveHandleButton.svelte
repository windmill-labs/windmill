<script lang="ts">
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import { onDestroy } from 'svelte'
	import { Move } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { MoveManager } from './moveManager.svelte'

	interface Props {
		moveManager: MoveManager
		moduleId: string
		selectedIds?: string[]
		singleNode?: boolean
		visible?: boolean
		onClickMove?: () => void
		class?: string
	}

	let {
		moveManager,
		moduleId,
		selectedIds,
		singleNode = false,
		visible = true,
		onClickMove,
		class: extraClass
	}: Props = $props()

	let dragCleanup: (() => void) | undefined

	function onPointerDown(e: Event) {
		const pe = e as PointerEvent
		const startX = pe.clientX
		const startY = pe.clientY
		let didDrag = false

		function onMove(me: PointerEvent) {
			const dx = me.clientX - startX
			const dy = me.clientY - startY
			if (!didDrag && Math.sqrt(dx * dx + dy * dy) > 5) {
				didDrag = true
				moveManager.startDrag(moduleId, startX, startY, singleNode ? undefined : selectedIds)
			}
		}

		function onUp() {
			document.removeEventListener('pointermove', onMove)
			document.removeEventListener('pointerup', onUp)
			dragCleanup = undefined
			if (!didDrag) {
				onClickMove?.()
			}
		}

		document.addEventListener('pointermove', onMove)
		document.addEventListener('pointerup', onUp)
		dragCleanup = onUp
	}

	onDestroy(() => dragCleanup?.())
</script>

<button
	class={twMerge(
		'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary cursor-grab',
		visible ? '' : '!hidden',
		'shadow-md rounded-md',
		'touch-none',
		extraClass
	)}
	onpointerdown={stopPropagation(preventDefault(onPointerDown))}
	title="Move"
>
	<Move size={12} />
</button>
