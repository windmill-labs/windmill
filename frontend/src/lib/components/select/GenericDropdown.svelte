<script lang="ts" generics="T">
	import { deepEqual } from 'fast-equals'
	import ConditionalPortal from '../common/drawer/ConditionalPortal.svelte'
	import { untrack, type Snippet } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { watch } from 'runed'

	let {
		listAutoWidth = true,
		strictWidth = false,
		disablePortal = false,
		open,
		disabled = false,
		class: className = '',
		innerClass = '',
		maxHeight = 256,
		getInputRect,
		children
	}: {
		listAutoWidth?: boolean
		strictWidth?: boolean
		disablePortal?: boolean
		open: boolean
		disabled?: boolean
		class?: string
		innerClass?: string
		maxHeight?: number
		getInputRect?: () => DOMRect
		children?: Snippet
	} = $props()

	let listEl: HTMLDivElement | undefined = $state()
	let dropdownPos = $state(computeDropdownPos())
	let reducedMotion = useReducedMotion()

	function computeDropdownPos(): {
		width: number
		height: number
		x: number
		y: number
		isBelow: boolean
	} {
		if (!getInputRect || !listEl) return { width: 0, height: 0, x: 0, y: 0, isBelow: true }
		let inputR = getInputRect()
		const listR = listEl.getBoundingClientRect()
		const fitsBelow = inputR.y + inputR.height + listR.height <= window.innerHeight
		const spaceBelow = window.innerHeight - (inputR.y + inputR.height)
		const spaceAbove = inputR.y
		const isBelow = fitsBelow || spaceBelow >= spaceAbove
		let [x, y] = disablePortal ? [0, 0] : [inputR.x, inputR.y]
		if (isBelow)
			return { width: inputR.width, height: listR.height, x: x, y: y + inputR.height, isBelow }
		else {
			return { width: inputR.width, height: listR.height, x: x, y: y - listR.height, isBelow }
		}
	}

	$effect(() => {
		function updateDropdownPos() {
			let nPos = computeDropdownPos()
			if (!deepEqual(nPos, dropdownPos)) dropdownPos = nPos
			if (open) requestAnimationFrame(updateDropdownPos)
		}
		if (open) untrack(() => updateDropdownPos())
	})

	// We do not want to render the dropdown when it is closed for performance reasons
	// but we want to keep it in the DOM for a short time to allow for transitions to finish
	//
	// We do not use Svelte transitions because they can not animate in the opposite direction
	// when the dropdown is opens above the input
	// Also CSS transitions are smoother because they do not rely on JS / animation frames
	let uiState = $state({ domExists: open, visible: open, timeout: null as number | null })
	let initial = true
	watch(
		() => open && !disabled,
		(isOpen) => {
			untrack(() => {
				if (initial) {
					initial = false
					return
				}
				if (reducedMotion.val) {
					uiState = {
						domExists: open && !disabled,
						visible: open && !disabled,
						timeout: null
					}
					return
				}
				if (uiState.timeout) clearTimeout(uiState.timeout)
				uiState = {
					domExists: true,
					visible: !isOpen,
					timeout: setTimeout(() => {
						if (isOpen) {
							uiState.visible = true
							uiState.timeout = null
						} else if (!isOpen) {
							uiState.visible = false
							uiState.timeout = setTimeout(() => {
								uiState.domExists = false
								uiState.timeout = null
							}, 500) // leave time for transition to finish
						}
					}, 0) // We need the height to be 0 then change immediately for the transition to play
				}
			})
		}
	)
</script>

<ConditionalPortal condition={!disablePortal} name="dropdown-portal">
	{#if uiState.domExists}
		<div
			class={twMerge(
				open ? 'dropdown-open' : 'dropdown-closed',
				disablePortal ? 'absolute z-[5002]' : 'fixed z-[10000]',
				'text-primary text-sm select-none',
				dropdownPos.isBelow ? '' : 'flex flex-col justify-end',
				uiState.visible ? '' : 'pointer-events-none',
				className
			)}
			style="{`top: ${dropdownPos.y}px; left: ${dropdownPos.x}px;`} {listAutoWidth
				? `${strictWidth ? 'width' : 'min-width'}: ${dropdownPos.width}px; height: ${dropdownPos.height}px;`
				: ''}"
		>
			<div
				class={twMerge(
					'overflow-clip rounded-md drop-shadow-base',
					!reducedMotion.val ? 'transition-height' : '',
					dropdownPos.isBelow ? '' : 'flex flex-col justify-end'
				)}
				style="height: {uiState.visible ? dropdownPos.height : 0}px;"
			>
				<div
					bind:this={listEl}
					class="flex flex-col rounded-md bg-surface-input {innerClass}"
					style="max-height: {maxHeight}px;"
				>
					{@render children?.()}
				</div>
			</div>
		</div>
	{/if}
</ConditionalPortal>
