<script lang="ts">
	import { preventDefault } from 'svelte/legacy'

	import { slide } from 'svelte/transition'
	import { ChevronDown } from 'lucide-svelte'
	import { isOpenStore } from './store'
	import { createEventDispatcher, onMount } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	interface Props {
		title: string
		prefix?: string | undefined
		openByDefault?: boolean
		wrapperClasses?: string
		toggleClasses?: string
		contentWrapperClasses?: string
		isOpen?: boolean
		tooltip?: string | undefined
		documentationLink?: string | undefined
		subtitle?: string | undefined
		titleSlot?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		title,
		prefix = undefined,
		openByDefault = false,
		wrapperClasses = '',
		toggleClasses = '',
		contentWrapperClasses = '',
		isOpen = $bindable(false),
		tooltip = undefined,
		documentationLink = undefined,
		subtitle = undefined,
		titleSlot,
		children
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let storeTitle = $derived(prefix + title)
	$effect(() => {
		isOpen = prefix ? $isOpenStore[storeTitle] : true
	})

	$effect(() => {
		dispatch('open', isOpen)
	})

	onMount(() => {
		if (prefix !== undefined && !(prefix + title in $isOpenStore)) {
			$isOpenStore[prefix + title] = openByDefault
		}
	})
</script>

<section class="pt-1 pb-2 px-1 {wrapperClasses}">
	{#if prefix !== undefined}
		<button
			onclick={preventDefault(() => isOpenStore.toggle(storeTitle))}
			class="w-full flex justify-between items-center text-secondary px-2 py-1
			rounded-sm duration-200 hover:bg-surface-hover {toggleClasses}"
		>
			<h1 class="text-sm font-semibold text-left">
				{#if titleSlot}{@render titleSlot()}{:else}
					{title}
					{#if subtitle}
						<span class="text-2xs text-tertiary ml-1">
							{subtitle}
						</span>
					{/if}
				{/if}
				{#if tooltip}
					<Tooltip class="ml-1" {documentationLink}>{tooltip}</Tooltip>
				{/if}
			</h1>
			<ChevronDown class="rotate-0 duration-300 {isOpen ? '!rotate-180' : ''}" />
		</button>
		{#if isOpen}
			<div transition:slide|local={{ duration: 300 }} class="px-2 {contentWrapperClasses}">
				{@render children?.()}
			</div>
		{/if}
	{:else}
		<h1 class="text-base font-semibold text-left px-2 py-1 text-secondary">
			{#if titleSlot}{@render titleSlot()}{:else}
				{title}
				{#if subtitle}
					<span class="text-2xs text-tertiary ml-1">
						{subtitle}
					</span>
				{/if}
			{/if}
			{#if tooltip}
				<Tooltip class="ml-1" {documentationLink}>{tooltip}</Tooltip>
			{/if}
		</h1>
		<div class="px-2">
			{@render children?.()}
		</div>
	{/if}
</section>
