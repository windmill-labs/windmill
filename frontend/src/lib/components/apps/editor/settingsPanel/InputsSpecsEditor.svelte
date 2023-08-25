<script lang="ts">
	import type { RichConfigurations } from '../../types'

	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import OneOfInputSpecsEditor from './OneOfInputSpecsEditor.svelte'

	export let id: string
	export let inputSpecs: RichConfigurations
	export let inputSpecsConfiguration: RichConfigurations | undefined = undefined
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let resourceOnly = false
	export let displayType = false

	$: finalInputSpecsConfiguration = inputSpecsConfiguration ?? inputSpecs
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(finalInputSpecsConfiguration) as k}
			{#if finalInputSpecsConfiguration[k]?.type == 'oneOf'}
				<OneOfInputSpecsEditor
					key={k}
					bind:oneOf={inputSpecs[k]}
					{id}
					{shouldCapitalize}
					{resourceOnly}
					inputSpecsConfiguration={finalInputSpecsConfiguration?.[k]?.['configuration']}
					labels={finalInputSpecsConfiguration?.[k]?.['labels']}
					tooltip={finalInputSpecsConfiguration?.[k]?.['tooltip']}
				/>
			{:else}
				{@const meta = finalInputSpecsConfiguration?.[k]}
				<InputsSpecEditor
					key={k}
					bind:componentInput={inputSpecs[k]}
					{id}
					{userInputEnabled}
					{shouldCapitalize}
					{resourceOnly}
					fieldType={meta?.['fieldType']}
					subFieldType={meta?.['subFieldType']}
					format={meta?.['format']}
					selectOptions={meta?.['selectOptions']}
					tooltip={meta?.['tooltip']}
					fileUpload={meta?.['fileUpload']}
					placeholder={meta?.['placeholder']}
					customTitle={meta?.['customTitle']}
					{displayType}
				/>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-tertiary text-sm">No inputs</div>
{/if}
