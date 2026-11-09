<script lang="ts">
	import { isMac } from '$lib/utils'
	import { isNewTabClick } from '$lib/navigation'
	import { onDestroy, onMount, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	// const dispatch = createEventDispatcher()

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})

	async function handleKeydown(event: KeyboardEvent) {
		if (hovered && event.key === 'Enter') {
			event.preventDefault()
			if (onkeyboardSpecificSelect) {
				onkeyboardSpecificSelect(event.shiftKey || event.ctrlKey)
			} else {
				onselect(event.shiftKey || event.ctrlKey)
			}
		}
	}

	interface Props {
		hovered?: boolean
		id: string
		label?: string
		/** Renders the item as a link, so ctrl/cmd-, middle- and right-click open it in a new tab. */
		href?: string
		icon?: any
		shortcutKey?: string | undefined
		containerClass?: string | undefined
		mouseMoved?: boolean
		kbdClass?: string
		small?: boolean
		itemReplacement?: import('svelte').Snippet
		onselect?: (shift: boolean) => void
		onkeyboardSpecificSelect?: (shift: boolean) => void
		onhover?: () => void
	}

	let {
		hovered = false,
		id,
		label = '',
		href = undefined,
		icon = undefined,
		shortcutKey = undefined,
		containerClass = undefined,
		mouseMoved = $bindable(false),
		kbdClass = $bindable(''),
		small = true,
		itemReplacement,
		onselect = () => {},
		onhover = () => {},
		onkeyboardSpecificSelect
	}: Props = $props()

	if (untrack(() => small)) {
		kbdClass = twMerge(
			kbdClass,
			'!text-[10px]  px-1',
			false && isMac() ? '!text-lg ' : 'text-xs',
			'leading-none'
		)
	} else {
		kbdClass += ' !text-xs px-1.5'
	}
</script>

<!-- Enter acts on the hovered item (window keydown), so the link stays out of the tab order:
     a focused link that isn't the hovered one would show a ring on the wrong item. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<svelte:element
	this={href ? 'a' : 'div'}
	{id}
	{href}
	tabindex={href ? -1 : undefined}
	onclickcapture={(e: MouseEvent) => {
		e.stopImmediatePropagation()
		// The link only serves clicks the browser opens elsewhere (modified, middle, right); a
		// plain click still goes through onselect, which may select rather than navigate.
		// Capture, not a delegated onclick: the palette is portalled outside the app root, where
		// a delegated handler runs after SvelteKit's link handler has already followed the href.
		if (href && isNewTabClick(e)) return
		if (href) e.preventDefault()
		onselect(e.shiftKey || e.ctrlKey)
	}}
	onmouseenter={() => {
		if (mouseMoved) {
			onhover()
		}
		mouseMoved = false
	}}
	class={twMerge(
		`block rounded-md w-full text-[color:inherit] transition-all cursor-pointer ${hovered ? 'bg-surface-hover' : ''}`,
		containerClass
	)}
>
	{#if itemReplacement}
		{@render itemReplacement?.()}
	{:else}
		<div class="flex flex-row gap-2 items-center px-2 py-1.5 rounded-md pr-6 text-sm">
			<div class="w-4">
				{#if icon}
					{@const SvelteComponent = icon}
					<SvelteComponent size={16} />
				{:else if shortcutKey != undefined}
					<div class="font-bold flex items-center justify-center w-full">
						<span
							class="h-4 center-center ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse"
						>
							<kbd class={kbdClass}>
								{shortcutKey}
							</kbd>
						</span>
					</div>
				{/if}
			</div>
			{label}
			{#if shortcutKey != undefined}
				<div class="ml-auto">
					<div class="font-bold flex items-center justify-center w-full">
						<span
							class="flex h-4 center-center ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse"
						>
							<kbd class={kbdClass}>
								{shortcutKey}
							</kbd>
						</span>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</svelte:element>
