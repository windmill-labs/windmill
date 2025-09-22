<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'

	interface Props {
		disabled?: boolean
		label: string
		lang?: SupportedLanguage | 'docker' | 'javascript' | undefined
		id?: string | undefined
	}

	let { disabled = false, label, lang = undefined, id = undefined }: Props = $props()

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql', 'oracledb']
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
	{#snippet text()}
		{label} is only available with an enterprise license
	{/snippet}
</Popover>
