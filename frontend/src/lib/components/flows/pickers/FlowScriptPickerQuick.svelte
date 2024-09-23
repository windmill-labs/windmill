<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'

	export let label: string
	export let lang: SupportedLanguage | 'docker' | 'javascript' | undefined = undefined

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql']
</script>

{#if !enterpriseLangs.includes(lang || '') || !!$enterpriseLicense}
	<button
		class="px-3 py-2 gap-2 w-full text-left hover:bg-surface-hover flex flex-row items-center transition-all rounded-md"
		on:click
		role="menuitem"
	>
		{#if lang}
			<LanguageIcon {lang} width={14} height={14} />
		{/if}
		<span class="grow truncate text-left text-xs text-primary font-semibold">
			{label}
		</span>
	</button>
{/if}
