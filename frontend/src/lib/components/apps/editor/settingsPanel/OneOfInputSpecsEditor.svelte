<script lang="ts">
	import Select from '$lib/components/select/Select.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { addWhitespaceBeforeCapitals, capitalize } from '$lib/utils'
	import type { RichConfiguration } from '../../types'
	import { cleanseOneOfConfiguration } from '../appUtils'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	interface Props {
		key: string
		oneOf: { selected: string; configuration: RichConfiguration } | any
		inputSpecsConfiguration: RichConfiguration | any
		labels: Record<string, string> | undefined
		shouldCapitalize: boolean
		id: string
		resourceOnly: boolean
		tooltip: string | undefined
		disabledOptions?: string[]
		acceptSelf?: boolean
		recomputeOnInputChanged?: boolean
		showOnDemandOnlyToggle?: boolean
	}

	let {
		key,
		oneOf = $bindable(),
		inputSpecsConfiguration,
		labels,
		shouldCapitalize,
		id,
		resourceOnly,
		tooltip,
		disabledOptions = [],
		acceptSelf = false,
		recomputeOnInputChanged = true,
		showOnDemandOnlyToggle = true
	}: Props = $props()

	$effect.pre(() => {
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
			let selected = Object.keys(inputSpecsConfiguration ?? {})[0]
			oneOf = {
				configuration: cleanseOneOfConfiguration(inputSpecsConfiguration),
				selected,
				type: 'oneOf'
			}
		}
		if (oneOf.selected && Object.keys(oneOf.configuration).length > 1) {
			oneOf = {
				...oneOf,
				configuration: {
					[oneOf.selected]: oneOf.configuration[oneOf.selected]
				}
			}
		}
	})

	function getValueOfDeprecated(obj: object): boolean {
		if (!obj) return false

		let innerObject = obj[Object.keys(obj)[0]]

		return innerObject?.deprecated
	}
</script>

<div class="p-2 border rounded-md">
	{#if oneOf}
		<div class="mb-2 text-xs font-semibold">
			{capitalize(addWhitespaceBeforeCapitals(key))}&nbsp;
			{#if tooltip}
				<Tooltip light>{tooltip}</Tooltip>
			{/if}
		</div>
		<Select
			bind:value={
				() => oneOf.selected,
				(selected) =>
					(oneOf = {
						...oneOf,
						configuration: { selected: inputSpecsConfiguration[selected] },
						selected
					})
			}
			items={Object.keys(inputSpecsConfiguration ?? {})
				.filter(
					(choice) =>
						(!disabledOptions.includes(choice) &&
							!getValueOfDeprecated(inputSpecsConfiguration[choice])) ||
						oneOf.selected === choice
				)
				.map((choice) => ({ label: labels?.[choice] ?? choice, value: choice }))}
		/>
		<!-- {JSON.stringify(inputSpecsConfiguration)} -->
		<!-- {JSON.stringify(oneOf.configuration)} -->

		{#if oneOf.selected !== 'none' && oneOf.selected !== 'errorOverlay'}
			<div class="mb-4"></div>
		{/if}
		<div class="flex flex-col gap-4">
			{#each Object.keys(inputSpecsConfiguration?.[oneOf.selected] ?? {}) as nestedKey}
				{@const config = {
					...inputSpecsConfiguration?.[oneOf.selected]?.[nestedKey],
					...oneOf.configuration?.[oneOf.selected]?.[nestedKey]
				}}

				{#if config && oneOf.configuration[oneOf.selected]}
					<InputsSpecEditor
						{recomputeOnInputChanged}
						key={nestedKey}
						bind:componentInput={oneOf.configuration[oneOf.selected][nestedKey]}
						{id}
						{acceptSelf}
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
						loading={config?.['loading']}
						documentationLink={config?.['documentationLink']}
						allowTypeChange={config?.['allowTypeChange']}
						{showOnDemandOnlyToggle}
					/>
				{/if}
			{/each}
		</div>
	{/if}
</div>
