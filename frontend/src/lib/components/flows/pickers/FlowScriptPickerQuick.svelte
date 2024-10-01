<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let label: string
	export let lang: SupportedLanguage | 'docker' | 'javascript' | undefined = undefined
	export let selected = false
	export let eeRestricted: boolean
	export let enterpriseLangs: string[] = []

	const dispatch = createEventDispatcher()
	function handleKeydown(event: KeyboardEvent & { currentTarget: EventTarget & Window }) {
		if (selected && event.key === 'Enter') {
			click()
		}
	}

	function click() {
		if (eeRestricted) {
			sendUserToast(
				`The languages ${enterpriseLangs.join(', ')} are only available on the enterprise edition`,
				true
			)
			return
		}
		dispatch('click')
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<button
	class={twMerge(
		'px-3 py-2 gap-2 w-full text-left hover:bg-surface-hover flex flex-row items-center transition-all rounded-md',
		selected ? 'bg-surface-hover' : ''
	)}
	on:click={click}
	role="menuitem"
>
	{#if lang}
		<LanguageIcon {lang} width={14} height={14} />
	{/if}
	<span
		class="grow truncate text-left text-2xs font-normal {eeRestricted
			? 'text-secondary'
			: 'text-primary'}"
	>
		{label}{#if eeRestricted}&nbsp;(EE){/if}
	</span>
	{#if selected}
		<kbd class="!text-xs">&crarr;</kbd>
	{/if}
</button>
