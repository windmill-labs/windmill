<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { addWhitespaceBeforeCapitals, capitalize } from '$lib/utils'
	import type { RichConfiguration } from '../../types'
	import { cleanseOneOfConfiguration } from '../appUtils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let key: string
	export let oneOf: { selected: string; configuration: RichConfiguration } | any
	export let inputSpecsConfiguration: RichConfiguration | any
	export let labels: Record<string, string> | undefined
	export let shouldCapitalize: boolean
	export let id: string
	export let resourceOnly: boolean
	export let tooltip: string | undefined

	$: {
		if (oneOf == undefined) {
			oneOf = { configuration: {}, selected: '' }
		}
		if (oneOf.configuration == undefined) {
			oneOf.configuration = {}
		}
		if (oneOf.selected == undefined) {
			oneOf.selected = ''
		}
		if (oneOf?.configuration[oneOf?.selected] == undefined) {
			oneOf.configuration[oneOf.selected] = {}
		}

		// If the configuration is empty, we set the first one as selected.
		// It happens when the configuration was added after the component was created
		if (oneOf.selected === '') {
			oneOf = {
				configuration: cleanseOneOfConfiguration(inputSpecsConfiguration),
				selected: Object.keys(inputSpecsConfiguration ?? {})[0],
				type: 'oneOf'
			}
		}
	}

	function getValueOfDeprecated(obj: object): boolean {
		if (!obj) return false

		let innerObject = obj[Object.keys(obj)[0]]

		return innerObject?.deprecated
	}
</script>

<div class="p-2 border">
	<div class="mb-2 text-sm font-semibold">
		{capitalize(addWhitespaceBeforeCapitals(key))}&nbsp;
		{#if tooltip}
			<Tooltip light>{tooltip}</Tooltip>
		{/if}
	</div>
	<select
		class="w-full border border-gray-300 rounded-md p-2"
		value={oneOf.selected}
		on:change={(e) => {
			oneOf = { ...oneOf, selected: e?.target?.['value'] }
		}}
	>
		{#each Object.keys(inputSpecsConfiguration ?? {}) as choice}
			{#if !getValueOfDeprecated(inputSpecsConfiguration[choice]) || oneOf.selected === choice}
				<option value={choice}>{labels?.[choice] ?? choice}</option>
			{/if}
		{/each}
	</select>
	{#if oneOf.selected !== 'none' && oneOf.selected !== 'errorOverlay'}
		<div class="mb-4" />
	{/if}
	<div class="flex flex-col gap-4">
		{#each Object.keys(inputSpecsConfiguration?.[oneOf.selected] ?? {}) as nestedKey}
			{@const config = inputSpecsConfiguration?.[oneOf.selected]?.[nestedKey]}
			{#if config && oneOf.configuration[oneOf.selected]}
				<InputsSpecEditor
					key={nestedKey}
					bind:componentInput={oneOf.configuration[oneOf.selected][nestedKey]}
					{id}
					userInputEnabled={false}
					{shouldCapitalize}
					{resourceOnly}
					fieldType={config?.['fieldType']}
					subFieldType={config?.['subFieldType']}
					format={config?.['format']}
					selectOptions={config?.['selectOptions']}
					placeholder={config?.['placeholder']}
					customTitle={config?.['customTitle']}
					tooltip={config?.['tooltip']}
					fileUpload={config?.['fileUpload']}
				/>
			{/if}
		{/each}
	</div>
</div>
