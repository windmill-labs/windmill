<script lang="ts">
	import type { RichConfigurations } from '../../types'

	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import OneOfInputSpecsEditor from './OneOfInputSpecsEditor.svelte'

	export let id: string
	export let inputSpecs: RichConfigurations
	export let inputSpecsConfiguration: RichConfigurations = inputSpecs
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let rowColumns = false
	export let resourceOnly = false
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecsConfiguration) as k}
			{#if inputSpecsConfiguration[k]?.type == 'oneOf'}
				<OneOfInputSpecsEditor
					key={k}
					bind:oneOf={inputSpecs[k]}
					{id}
					{shouldCapitalize}
					{resourceOnly}
					{rowColumns}
					inputSpecsConfiguration={inputSpecsConfiguration?.[k]?.['configuration']}
					labels={inputSpecsConfiguration?.[k]?.['labels']}
					tooltip={inputSpecsConfiguration?.[k]?.['tooltip']}
				/>
			{:else}
				{@const meta = inputSpecsConfiguration?.[k]}
				<InputsSpecEditor
					key={k}
					bind:componentInput={inputSpecs[k]}
					{id}
					{userInputEnabled}
					{shouldCapitalize}
					{resourceOnly}
					hasRows={rowColumns}
					fieldType={meta?.['fieldType']}
					subFieldType={meta?.['subFieldType']}
					format={meta?.['format']}
					selectOptions={meta?.['selectOptions']}
					tooltip={meta?.['tooltip']}
					onlyStatic={meta?.['onlyStatic']}
					fileUpload={meta?.['fileUpload']}
					placeholder={meta?.['placeholder']}
				/>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
