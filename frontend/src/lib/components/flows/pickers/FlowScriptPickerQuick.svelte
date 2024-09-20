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
		class="w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center"
		on:click
		role="menuitem"
	>
		<div class="flex justify-center flex-row items-center gap-2">
			{#if lang}
				<LanguageIcon {lang} width={14} height={14} />
			{/if}
			<span class="text-xs">{label}</span>
		</div>
	</button>
{/if}
