<svelte:options accessors />

<script lang="ts">
	import { onDestroy } from 'svelte'
	import { slide, type TransitionConfig,  } from 'svelte/transition'
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import { clickOutside } from '../../../utils'
	import { createStateMachine } from '../../../stateMachine'

	export let ref: HTMLElement | undefined
	export let options: PopperOptions<any> = { placement: 'auto', strategy: 'fixed' }
	/** Events on the reference element */
	export let openOn: (keyof HTMLElementEventMap)[] = ['focus']
	/** Events on the reference element */
	export let closeOn: (keyof HTMLElementEventMap)[] = ['blur']
	export let innerClasses = ''
	export let outerClasses = ''
	export let transition: (node: Element, params?: Record<string, any>) => TransitionConfig = slide

	const states = ['closed', 'open-focus-in', 'open-focus-out'] as const
	const stateMachine = createStateMachine(states, {
		to: {
			closed: ({ previousState, currentState }) => {
				return isFocusContained() ? previousState : currentState
			}
		}
	})

	const [popperRef, popperContent, getInstance] = createPopperActions()
	let popup: HTMLElement | undefined
	let focusableElements: HTMLElement[]

	function getFocusableElements() {
		let elements: HTMLElement[] = []

		popup
			?.querySelectorAll<HTMLElement>(
				'a[href], button, input, textarea, select, details, [tabindex]:not([tabindex="-1"])'
			)
			.forEach((elem) => elements.push(elem))

		focusableElements = elements.filter(
			(el) => !el.hasAttribute('disabled') && !el.getAttribute('aria-hidden')
		)
		focusableElements.forEach((el) => {
			el.tabIndex = -1
			el.addEventListener('click', openFocusIn)
			el.addEventListener('blur', conditionalClosed)
		})
	}

	function isFocusContained() {
		const activeElem = document.activeElement
		return popup?.contains(activeElem) || ref?.contains(activeElem)
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
	function conditionalClosed() {
		if(isFocusContained()) return;
		closed()
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
		if (modifiers.includes(event.key) || $stateMachine.currentState === 'closed') return;
		if (event.key === 'Escape') {
			return (<HTMLElement>document.activeElement)?.blur()
		}
		
		const prev = ['Up', 'Left']
		const next = ['Down', 'Right']
		if (![...prev, ...next].some(dir => `Arrow${dir}` === event.key)) return;

		event.preventDefault()

		if (popup?.contains(document.activeElement)) {
			const index = focusableElements.findIndex((elem) => elem === document.activeElement)
			if (index === -1) return;

			let targetIndex: number | undefined = undefined
			if (prev.some(dir => `Arrow${dir}` === event.key)) {
				targetIndex = index === 0 ? focusableElements.length - 1 : index - 1
			} else if (next.some(dir => `Arrow${dir}` === event.key)) {
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
		if(!ref) return;
		openOn.forEach((action) => ref!.addEventListener(action, openFocusOut))
		closeOn.forEach((action) => ref!.addEventListener(action, closed))
	}

	function removeAllListeners() {
		focusableElements?.forEach((el) => el.removeEventListener('click', openFocusIn))
		focusableElements?.forEach((el) => el.removeEventListener('blur', conditionalClosed))
		if(!ref) return;
		openOn.forEach((action) => ref!.removeEventListener(action, openFocusOut))
		closeOn.forEach((action) => ref!.removeEventListener(action, closed))
	}

	$: if ($stateMachine.currentState === 'closed') {
		focusableElements?.forEach((el) => el.removeEventListener('click', openFocusIn))
	} else {
		setTimeout(() => {
			getFocusableElements()
		}, 0)
	}

	$: if(ref) {
		popperRef(ref)
		addRefListeners()
	}

	$: $$slots.default && getInstance()?.update()

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
		<div transition:transition|local={{ duration: 200 }} class={outerClasses}>
			<div class={innerClasses}>
				<slot />
			</div>
		</div>
	{/if}
</div>
