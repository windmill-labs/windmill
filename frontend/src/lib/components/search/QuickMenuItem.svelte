<script lang="ts">
	import { isMac } from '$lib/utils'
	import { onDestroy, onMount } from 'svelte'
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

	if (small) {
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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	{id}
	onclick={(e) => {
		e.stopImmediatePropagation()
		onselect(e.shiftKey || e.ctrlKey)
	}}
	onmouseenter={() => {
		if (mouseMoved) {
			onhover()
		}
		mouseMoved = false
	}}
	class={twMerge(
		`rounded-md w-full transition-all cursor-pointer ${hovered ? 'bg-surface-hover' : ''}`,
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
</div>
