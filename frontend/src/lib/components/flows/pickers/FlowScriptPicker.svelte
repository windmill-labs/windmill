<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import type { IconDefinition } from '@fortawesome/free-solid-svg-icons'

	export let disabled: boolean = false
	export let label: string
	export let lang: SupportedLanguage | 'pgsql' | 'mysql' | 'javascript' | 'fetch' | undefined =
		undefined
	export let icon: IconDefinition | undefined = undefined
	export let iconColor: string | undefined = undefined
</script>

<Button
	{disabled}
	btnClasses="w-24 truncate"
	on:click
	size="sm"
	spacingSize="md"
	variant="border"
	color="light"
	startIcon={{
		icon,
		classes: iconColor
	}}
>
	<div class="flex justify-center flex-col items-center gap-2">
		{#if lang}
			<LanguageIcon {lang} />
		{/if}
		<span class="text-xs">{label}</span>
	</div>
</Button>
