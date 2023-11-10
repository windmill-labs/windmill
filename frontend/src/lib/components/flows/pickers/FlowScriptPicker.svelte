<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'

	export let disabled: boolean = false
	export let label: string
	export let lang:
		| SupportedLanguage
		| 'pgsql'
		| 'mysql'
		| 'javascript'
		| 'fetch'
		| 'docker'
		| 'powershell'
		| undefined = undefined

	export let id: string | undefined = undefined

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql']
</script>

<Popover disablePopup={!enterpriseLangs.includes(lang || '') || !!$enterpriseLicense}>
	<Button
		btnClasses="w-32 truncate"
		on:click
		size="sm"
		spacingSize="md"
		variant="border"
		color="light"
		disabled={disabled || (enterpriseLangs.includes(lang || '') && !$enterpriseLicense)}
		{id}
	>
		<div class="flex justify-center flex-col items-center gap-2">
			{#if lang}
				<LanguageIcon {lang} />
			{/if}
			<span class="text-xs">{label}</span>
		</div>
	</Button>
	<svelte:fragment slot="text">{label} is only available with an enterprise license</svelte:fragment
	>
</Popover>
