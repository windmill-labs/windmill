<svelte:options accessors />

<script lang="ts">
	import { slide } from 'svelte/transition'
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import { clickOutside } from '../../../utils'
	import { Kbd } from '..'

	export let options: PopperOptions<any> = { placement: 'auto' }
	const [popperRef, popperContent] = createPopperActions()
	export let isOpen = false
	export let isFocused = false
	export let disableInstruction = false

	export const ref = popperRef
	export const toggle = () => (isOpen ? close() : open())
	export const open = () => (isOpen = true)
	export const close = () => (isOpen = isFocused = false)

	let element: HTMLElement
	let focusableElements: HTMLElement[]

	function getFocusableElements() {
		let elements: HTMLElement[] = []

		element
			.querySelectorAll<HTMLElement>(
				'a[href], button, input, textarea, select, details, [tabindex]:not([tabindex="-1"])'
			)
			.forEach((elem) => elements.push(elem))

		focusableElements = elements.filter(
			(el) => !el.hasAttribute('disabled') && !el.getAttribute('aria-hidden')
		)
	}

	function removeTabTargeting() {
		focusableElements.forEach((el) => (el.tabIndex = -1))
	}

	function keyDown(event: KeyboardEvent & { currentTarget: EventTarget & Window }) {
		if (
			!isOpen ||
			!focusableElements?.length ||
			(event.key !== 'ArrowUp' && event.key !== 'ArrowDown')
		) {
			return close()
		}
		event.preventDefault()

		if (element.contains(document.activeElement)) {
			const index = focusableElements.findIndex((elem) => elem === document.activeElement)
			if (index === -1) return

			let targetIndex: number | undefined = undefined
			if (event.key === 'ArrowUp') {
				targetIndex = index === 0 ? focusableElements.length - 1 : index - 1
			} else if (event.key === 'ArrowDown') {
				targetIndex = index + 1 === focusableElements.length ? 0 : index + 1
			}
			if (targetIndex !== undefined) {
				focusableElements[targetIndex].focus()
				isFocused = true
			}
		} else {
			focusableElements[event.key === 'ArrowUp' ? focusableElements.length - 1 : 0]?.focus()
			isFocused = true
		}
	}

	$: if (element) {
		getFocusableElements()
		removeTabTargeting()
	}
</script>

<svelte:window on:keydown={keyDown} />

{#if isOpen}
	<div
		bind:this={element}
		transition:slide={{ duration: 200 }}
		use:popperContent={options}
		use:clickOutside
		on:click_outside={close}
		class={$$props.class}
	>
		{#if !disableInstruction && focusableElements?.length}
			<div
				class="flex justify-center items-center font-semibold 
				text-xs text-gray-600 p-1 rounded-t bg-gray-50"
			>
				Use
				<Kbd class="!px-1 !py-0 !rounded-sm">↑</Kbd>
				and
				<Kbd class="!px-1 !py-0 !rounded-sm">↓</Kbd>
			</div>
		{/if}
		<slot />
	</div>
{/if}
