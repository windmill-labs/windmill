<svelte:options accessors />

<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { slide } from 'svelte/transition'
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import { clickOutside } from '../../../utils'
	import { Kbd } from '..'
	import { createStateMachine } from '../../../stateMachine'

	export let ref: HTMLElement
	export let options: PopperOptions<any> = { placement: 'auto' }
	/** Events on the reference element */
	export let openOn: (keyof HTMLElementEventMap)[] = ['focus']
	/** Events on the reference element */
	export let closeOn: (keyof HTMLElementEventMap)[] = ['blur']
	export let disableInstruction = false
	export let innerClasses = ''
	export let outerClasses = ''

	const states = ['closed', 'open-focus-in', 'open-focus-out'] as const
	const stateMachine = createStateMachine(states, {
		to: {
			closed: ({ previousState, currentState }) => {
				const activeElem = document.activeElement
				const revert = popup.contains(activeElem) || ref.contains(activeElem)
				return revert ? previousState : currentState
			}
		}
	})

	const [popperRef, popperContent] = createPopperActions()
	let popup: HTMLElement
	let focusableElements: HTMLElement[]

	function getFocusableElements() {
		let elements: HTMLElement[] = []

		popup
			.querySelectorAll<HTMLElement>(
				'a[href], button, input, textarea, select, details, [tabindex]:not([tabindex="-1"])'
			)
			.forEach((elem) => elements.push(elem))

		focusableElements = elements.filter(
			(el) => !el.hasAttribute('disabled') && !el.getAttribute('aria-hidden')
		)
		focusableElements.forEach((el) => {
			el.tabIndex = -1
			el.addEventListener('click', openFocusIn)
			el.addEventListener('blur', closed)
		})
	}

	function closed() {
		if ($stateMachine.currentState === 'open-focus-out') {
			setTimeout(() => {
				stateMachine.setState('closed')
			}, 0)
		} else {
			stateMachine.setState('closed')
		}
	}
	function openFocusOut() {
		stateMachine.setState('open-focus-out')
	}
	function openFocusIn() {
		stateMachine.setState('open-focus-in')
	}

	function keyDown(event: KeyboardEvent & { currentTarget: EventTarget & Window }) {
		const modifiers = ['Shift', 'Control', 'Command', 'Alt']
		// Prevent closing the popup when the only key pressed is a modifier key
		if (modifiers.includes(event.key) || $stateMachine.currentState === 'closed') return
		if (event.key === 'Escape') {
			return (<HTMLElement>document.activeElement)?.blur()
		}
		if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return

		event.preventDefault()

		if (popup.contains(document.activeElement)) {
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
				stateMachine.setState('open-focus-in')
			}
		} else {
			const elem = focusableElements[event.key === 'ArrowUp' ? focusableElements.length - 1 : 0]
			if (elem) {
				elem.focus()
				stateMachine.setState('open-focus-in')
			}
		}
	}

	function addRefListeners() {
		openOn.forEach((action) => ref.addEventListener(action, openFocusOut))
		closeOn.forEach((action) => ref.addEventListener(action, closed))
	}

	function removeAllListeners() {
		focusableElements?.forEach((el) => el.removeEventListener('click', openFocusIn))
		focusableElements?.forEach((el) => el.removeEventListener('blur', closed))
		openOn.forEach((action) => ref.removeEventListener(action, openFocusOut))
		closeOn.forEach((action) => ref.removeEventListener(action, closed))
	}

	$: if ($stateMachine.currentState === 'closed') {
		focusableElements?.forEach((el) => el.removeEventListener('click', openFocusIn))
	} else {
		setTimeout(() => {
			getFocusableElements()
		}, 0)
	}

	onMount(() => {
		popperRef(ref)
		addRefListeners()
	})

	onDestroy(removeAllListeners)
</script>

<svelte:window on:keydown={keyDown} />

<div
	bind:this={popup}
	use:popperContent={options}
	use:clickOutside
	on:click_outside={closed}
	aria-haspopup="true"
	aria-expanded={$stateMachine.currentState !== 'closed'}
>
	{#if $stateMachine.currentState !== 'closed'}
		<div transition:slide={{ duration: 200 }} class={outerClasses}>
			<div class={innerClasses}>
				<slot />
			</div>
		</div>
	{/if}
</div>
