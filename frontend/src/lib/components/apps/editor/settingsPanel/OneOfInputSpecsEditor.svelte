<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { RichConfiguration } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let key: string
	export let oneOf: { selected: string; configuration: RichConfiguration } | any
	export let inputSpecsConfiguration: RichConfiguration | any
	export let labels: Record<string, string> | undefined
	export let shouldCapitalize: boolean
	export let id: string
	export let resourceOnly: boolean
	export let rowColumns: boolean
	export let tooltip: string | undefined

	$: if (oneOf.configuration[oneOf.selected] == undefined) {
		oneOf.configuration[oneOf.selected] = {}
	}
</script>

<div class="p-2 border">
	<h4 class="mb-2">{key}&nbsp;<Tooltip>{tooltip}</Tooltip></h4>
	<select
		class="w-full border border-gray-300 rounded-md p-2"
		value={oneOf.selected}
		on:change={(e) => {
			oneOf = { ...oneOf, selected: e?.target?.['value'] }
		}}
	>
		{#each Object.keys(inputSpecsConfiguration ?? {}) as choice}
			<option value={choice}>{labels?.[choice] ?? choice}</option>
		{/each}
	</select>
	<div class="mb-4" />
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
					hasRows={rowColumns}
					fieldType={config?.['fieldType']}
					subFieldType={config?.['subFieldType']}
					format={config?.['format']}
					selectOptions={config?.['selectOptions']}
					placeholder={config?.['placeholder']}
				/>
			{/if}
		{/each}
	</div>
</div>
