<script lang="ts" module>
	let current: (() => void) | undefined = undefined
</script>

<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, onMount } from 'svelte'
	import { fade } from 'svelte/transition'

	let menu: HTMLDivElement | undefined = $state()

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	interface Props {
		noMinW?: boolean
		show?: boolean
		wrapperClasses?: string
		popupClasses?: string
		transitionDuration?: number
		pointerDown?: boolean
		placement?: Placement
		trigger?: import('svelte').Snippet<[any]>
		children?: import('svelte').Snippet<[any]>
	}

	let {
		noMinW = false,
		show = $bindable(false),
		wrapperClasses = '',
		popupClasses = '',
		transitionDuration = 25,
		pointerDown = false,
		placement = 'bottom-start',
		trigger,
		children
	}: Props = $props()

	function handleOutsideClick(event) {
		if (show && !menu?.contains(event.target)) {
			show = false
			event.preventDefault()
			event.stopPropagation()
		}
	}

	function handleEscape(event) {
		if (show && event.key === 'Escape') {
			show = false
		}
	}

	function close() {
		show = false
	}

	onMount(() => {
		document.addEventListener('click', handleOutsideClick, false)
		document.addEventListener('keyup', handleEscape, false)

		return () => {
			document.removeEventListener('click', handleOutsideClick, false)
			document.removeEventListener('keyup', handleEscape, false)
		}
	})

	const placementsClasses = {
		'bottom-center': 'origin-top-left left-1/2 transform -translate-x-1/2',
		'bottom-start': 'origin-top-left left-0',
		'bottom-end': 'origin-top-right right-0',
		'top-start': 'origin-bottom-left left-0 bottom-0',
		'top-end': 'origin-bottom-right right-0 bottom-0',
		'top-center': 'origin-top-left -top-full left-1/2  transform -translate-x-1/2 -translate-y-full'
	}
	const dispatch = createEventDispatcher()
</script>

<div class="relative {wrapperClasses}" bind:this={menu}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onclick={() => {
			if (!pointerDown) {
				if (!show) {
					current && current()
					current = close
				}
				show = !show
				if (show) {
					dispatch('dropdownOpen')
				} else {
					dispatch('dropdownClose')
				}
			}
		}}
		onpointerdown={() => {
			if (pointerDown) {
				if (!show) {
					current && current()
					current = close
				}
				show = !show
			}
		}}
		class="relative"
	>
		{@render trigger?.({ class: 'triggerable' })}
	</div>
	{#if show}
		<div
			transition:fade|local={{ duration: transitionDuration }}
			class={classNames(
				'z-50 absolute mt-2 rounded-md shadow-lg bg-surface ring-1 ring-black ring-opacity-5 focus:outline-none',
				placementsClasses[placement],
				noMinW ? 'min-w-0' : 'w-60',
				popupClasses
			)}
			role="menu"
			tabindex="-1"
		>
			{@render children?.({ close })}
		</div>
	{/if}
</div>
