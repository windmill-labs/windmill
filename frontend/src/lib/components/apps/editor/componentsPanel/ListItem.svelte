<script lang="ts">
	import { slide } from 'svelte/transition'
	import { ChevronDown } from 'lucide-svelte'
	import { isOpenStore } from './store'
	import { createEventDispatcher, onMount } from 'svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let title: string
	export let prefix: string | undefined = undefined
	export let openByDefault: boolean = false
	export let wrapperClasses = ''
	export let toggleClasses = ''
	export let contentWrapperClasses = ''
	export let isOpen = false
	export let tooltip: string | undefined = undefined
	export let documentationLink: string | undefined = undefined
	export let subtitle: string | undefined = undefined

	const dispatch = createEventDispatcher()

	$: storeTitle = prefix + title
	$: isOpen = prefix ? $isOpenStore[storeTitle] : true

	$: dispatch('open', isOpen)

	onMount(() => {
		if (prefix !== undefined && !(prefix + title in $isOpenStore)) {
			$isOpenStore[prefix + title] = openByDefault
		}
	})
</script>

<section class="pt-1 pb-2 px-1 {wrapperClasses}">
	{#if prefix !== undefined}
		<button
			on:click|preventDefault={() => isOpenStore.toggle(storeTitle)}
			class="w-full flex justify-between items-center text-secondary px-2 py-1
			rounded-sm duration-200 hover:bg-surface-hover {toggleClasses}"
		>
			<h1 class="text-sm font-semibold text-left">
				<slot name="title">
					{title}
					{#if subtitle}
						<span class="text-2xs text-tertiary ml-1">
							{subtitle}
						</span>
					{/if}
				</slot>
				{#if tooltip}
					<Tooltip class="ml-1" {documentationLink}>{tooltip}</Tooltip>
				{/if}
			</h1>
			<ChevronDown class="rotate-0 duration-300 {isOpen ? '!rotate-180' : ''}" />
		</button>
		{#if isOpen}
			<div transition:slide|local={{ duration: 300 }} class="px-2 {contentWrapperClasses}">
				<slot />
			</div>
		{/if}
	{:else}
		<h1 class="text-base font-semibold text-left px-2 py-1 text-secondary">
			<slot name="title">
				{title}
				{#if subtitle}
					<span class="text-2xs text-tertiary ml-1">
						{subtitle}
					</span>
				{/if}
			</slot>
			{#if tooltip}
				<Tooltip class="ml-1" {documentationLink}>{tooltip}</Tooltip>
			{/if}
		</h1>
		<div class="px-2">
			<slot />
		</div>
	{/if}
</section>
