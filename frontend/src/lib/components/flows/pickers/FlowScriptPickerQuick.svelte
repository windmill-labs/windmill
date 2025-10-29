<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'

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

<Button
	id={`flow-editor-new-${lang}`}
	{selected}
	onClick={click}
	role="menuitem"
	variant="subtle"
	unifiedSize="sm"
	btnClasses="justify-start"
>
	{#if lang}
		<LanguageIcon {lang} width={13} height={13} />
	{/if}
	<span class="grow truncate text-left {eeRestricted ? 'text-disabled' : ''}">
		{label}{#if eeRestricted}&nbsp;(EE){/if}
	</span>
	{#if selected}
		<kbd class="!text-xs">&crarr;</kbd>
	{/if}
</Button>
