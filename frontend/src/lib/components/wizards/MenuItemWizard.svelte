<script lang="ts">
	import { Popup } from '../common'

	import type { AppViewerContext, RichConfiguration } from '../apps/types'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import { getContext } from 'svelte'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	type MenuItem = {
		label: RichConfiguration
		icon: RichConfiguration
		href: RichConfiguration
	}

	export let value: MenuItem | undefined
</script>

<Popup
	floatingConfig={{ strategy: 'fixed', placement: 'left-end' }}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<div class="flex flex-col w-96 p-2 gap-4">
			<InputsSpecEditor
				key={'Data'}
				bind:componentInput={value.label}
				id={$selectedComponent?.[0] ?? ''}
				userInputEnabled={false}
				shouldCapitalize={true}
				resourceOnly={false}
				fieldType={value.label?.['fieldType']}
				subFieldType={value.label?.['subFieldType']}
				format={value.label?.['format']}
				selectOptions={value.label?.['selectOptions']}
				tooltip={value.label?.['tooltip']}
				fileUpload={value.label?.['fileUpload']}
				placeholder={value.label?.['placeholder']}
				customTitle={value.label?.['customTitle']}
				displayType={false}
			/>
		</div>
	{/if}
</Popup>
