<script lang="ts">
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { createEventDispatcher } from 'svelte'
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
	export let deletable = false
	export let acceptSelf: boolean = false

	$: finalInputSpecsConfiguration = inputSpecsConfiguration ?? inputSpecs

	const dispatch = createEventDispatcher()
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(finalInputSpecsConfiguration) as k}
			{#if finalInputSpecsConfiguration[k]?.type == 'oneOf'}
				<OneOfInputSpecsEditor
					{acceptSelf}
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
					{acceptSelf}
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
					loading={meta?.['loading']}
					{displayType}
				/>
				{#if deletable}
					<div class="flex flex-row-reverse -mt-4">
						<CloseButton noBg on:close={() => dispatch('delete', k)} />
					</div>{/if}
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-tertiary text-sm">No inputs</div>
{/if}
