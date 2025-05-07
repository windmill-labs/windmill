<script lang="ts">
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { createEventDispatcher } from 'svelte'
	import type { RichConfigurations } from '../../types'

	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import OneOfInputSpecsEditor from './OneOfInputSpecsEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let id: string
	export let inputSpecs: RichConfigurations
	export let inputSpecsConfiguration: RichConfigurations | undefined = undefined
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let resourceOnly = false
	export let displayType = false
	export let deletable = false
	export let acceptSelf: boolean = false
	export let recomputeOnInputChanged = true
	export let showOnDemandOnlyToggle = false
	export let securedContext = false
	export let overridenByComponent: string[] = []

	$: finalInputSpecsConfiguration = inputSpecsConfiguration ?? inputSpecs

	const dispatch = createEventDispatcher()

	const mapping = {
		onSuccess: 'On success wizard',
		onError: 'On error wizard'
	}
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(finalInputSpecsConfiguration) as k}
			{#if overridenByComponent.includes(k)}
				<div>
					<span class="text-xs font-semibold truncate text-primary">
						{k}
					</span>
					<div class="text-tertiary text-xs">Managed by the component</div>
				</div>
			{:else if finalInputSpecsConfiguration[k]?.type == 'oneOf'}
				<OneOfInputSpecsEditor
					{acceptSelf}
					key={mapping[k] ?? k}
					bind:oneOf={inputSpecs[k]}
					{id}
					{shouldCapitalize}
					{resourceOnly}
					inputSpecsConfiguration={finalInputSpecsConfiguration?.[k]?.['configuration']}
					labels={finalInputSpecsConfiguration?.[k]?.['labels']}
					tooltip={finalInputSpecsConfiguration?.[k]?.['tooltip']}
					{recomputeOnInputChanged}
					{showOnDemandOnlyToggle}
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
					fileUploadS3={meta?.['fileUploadS3']}
					placeholder={meta?.['placeholder']}
					customTitle={meta?.['customTitle']}
					loading={meta?.['loading']}
					documentationLink={meta?.['documentationLink']}
					markdownTooltip={meta?.['markdownTooltip']}
					allowTypeChange={meta?.['allowTypeChange']}
					{displayType}
					{recomputeOnInputChanged}
					{showOnDemandOnlyToggle}
					{securedContext}
				/>
				{#if deletable}
					<div class="flex flex-row-reverse -mt-4">
						<CloseButton noBg on:close={() => dispatch('delete', k)} />
					</div>{/if}
			{/if}
		{/each}
		{#if overridenByComponent.length > 0}
			{#each overridenByComponent.filter((item) => Object.keys(finalInputSpecsConfiguration).indexOf(item) < 0) as k}
				<div>
					<span class="text-xs font-semibold truncate text-primary">
						{k}
					</span>
					<div class="text-tertiary text-xs">
						Managed by the component
						<Tooltip light>
							The input is managed by the component and cannot be edited here: It will be injected
							by the component itself.
						</Tooltip>
					</div>
				</div>
			{/each}
		{/if}
	</div>
{:else}
	<div class="text-tertiary text-sm">No inputs</div>
{/if}
