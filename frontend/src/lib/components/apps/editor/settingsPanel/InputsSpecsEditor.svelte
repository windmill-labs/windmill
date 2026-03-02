<script lang="ts">
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { createEventDispatcher } from 'svelte'
	import type { RichConfigurations } from '../../types'

	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import OneOfInputSpecsEditor from './OneOfInputSpecsEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	interface Props {
		id: string
		inputSpecs: RichConfigurations
		inputSpecsConfiguration?: RichConfigurations | undefined
		userInputEnabled?: boolean
		shouldCapitalize?: boolean
		resourceOnly?: boolean
		displayType?: boolean
		deletable?: boolean
		acceptSelf?: boolean
		recomputeOnInputChanged?: boolean
		showOnDemandOnlyToggle?: boolean
		securedContext?: boolean
		overridenByComponent?: string[]
	}

	let {
		id,
		inputSpecs = $bindable(),
		inputSpecsConfiguration = undefined,
		userInputEnabled = false,
		shouldCapitalize = true,
		resourceOnly = false,
		displayType = false,
		deletable = false,
		acceptSelf = false,
		recomputeOnInputChanged = true,
		showOnDemandOnlyToggle = false,
		securedContext = false,
		overridenByComponent = []
	}: Props = $props()

	let finalInputSpecsConfiguration = $derived(inputSpecsConfiguration ?? inputSpecs)

	const dispatch = createEventDispatcher()

	const mapping = {
		onSuccess: 'On success wizard',
		onSubmit: 'On submit wizard',
		onError: 'On error wizard'
	}
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(finalInputSpecsConfiguration) as k (k)}
			{#if overridenByComponent.includes(k)}
				<div>
					<span class="text-xs font-semibold truncate text-primary">
						{k}
					</span>
					<div class="text-primary text-xs">Managed by the component</div>
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
		allowTypeChange={meta?.['allowTypeChange']}
					{displayType}
					{recomputeOnInputChanged}
					{showOnDemandOnlyToggle}
					{securedContext}
				/>
				{#if deletable}
					<div class="flex flex-row-reverse -mt-4">
						<CloseButton noBg on:close={() => dispatch('delete', k)} />
					</div>
				{/if}
			{/if}
		{/each}
		{#if overridenByComponent.length > 0}
			{#each overridenByComponent.filter((item) => Object.keys(finalInputSpecsConfiguration).indexOf(item) < 0) as k}
				<div>
					<span class="text-xs font-semibold truncate text-primary">
						{k}
					</span>
					<div class="text-primary text-xs">
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
	<div class="text-primary text-xs">No inputs</div>
{/if}
