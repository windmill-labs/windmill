<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let label: string
	export let lang: SupportedLanguage | 'docker' | 'javascript' | undefined = undefined
	export let selected = false

	const dispatch = createEventDispatcher()
	function handleKeydown(event: KeyboardEvent & { currentTarget: EventTarget & Window }) {
		if (selected && event.key === 'Enter') {
			dispatch('click')
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<button
	class={twMerge(
		'px-3 py-2 gap-2 w-full text-left hover:bg-surface-hover flex flex-row items-center transition-all rounded-md',
		selected ? 'bg-surface-hover' : ''
	)}
	on:click
	role="menuitem"
>
	{#if lang}
		<LanguageIcon {lang} width={14} height={14} />
	{/if}
	<span class="grow truncate text-left text-2xs text-primary font-normal">
		{label}
	</span>
	{#if selected}
		<kbd class="!text-xs">&crarr;</kbd>
	{/if}
</button>
