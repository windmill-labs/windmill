<script lang="ts">
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import type { IconDefinition } from '@fortawesome/free-solid-svg-icons'

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
	export let icon: IconDefinition | undefined = undefined
	export let iconColor: string | undefined = undefined

	const enterpriseLangs = ['bigquery', 'snowflake']
</script>

<Popover disablePopup={!enterpriseLangs.includes(lang || '') || !!$enterpriseLicense}>
	<Button
		btnClasses="w-32 truncate"
		on:click
		size="sm"
		spacingSize="md"
		variant="border"
		color="light"
		startIcon={{
			icon,
			classes: iconColor
		}}
		disabled={disabled || (enterpriseLangs.includes(lang || '') && !$enterpriseLicense)}
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
