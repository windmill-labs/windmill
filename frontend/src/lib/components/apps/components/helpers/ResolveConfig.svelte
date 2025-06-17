<script lang="ts">
	import InputValue from './InputValue.svelte'
	import type { RichConfiguration } from '../../types'
	import { untrack } from 'svelte'

	interface Props {
		id: string
		extraKey?: string
		key: string
		resolvedConfig: any | { type: 'oneOf'; configuration: any; selected: string }
		configuration: RichConfiguration
		initialConfig?: RichConfiguration | undefined
	}

	let {
		id,
		extraKey = '',
		key,
		resolvedConfig = $bindable(),
		configuration,
		initialConfig = undefined
	}: Props = $props()

	function handleSelected(selected: string) {
		if (resolvedConfig?.selected != undefined && resolvedConfig?.selected != selected) {
			resolvedConfig.selected = selected
		}
	}
	$effect(() => {
		configuration?.type == 'oneOf' &&
			configuration.selected &&
			untrack(() => handleSelected(configuration.selected))
	})
</script>

{#if configuration?.type == 'oneOf' && resolvedConfig?.type == 'oneOf'}
	{@const choice = resolvedConfig.selected}
	{#each Object.keys(configuration.configuration?.[choice] ?? {}) as nestedKey (nestedKey)}
		{#if resolvedConfig.configuration?.[choice] != undefined}
			<InputValue
				field={nestedKey}
				key={key + choice + nestedKey + extraKey}
				{id}
				input={configuration?.configuration?.[choice]?.[nestedKey]}
				bind:value={resolvedConfig.configuration[choice][nestedKey]}
				onDemandOnly={initialConfig?.type == 'oneOf' &&
					initialConfig?.configuration?.[choice]?.[nestedKey]?.onDemandOnly}
				exportValueFunction
			/>
		{/if}
	{/each}
{:else}
	<InputValue
		field={key}
		key={key + extraKey}
		{id}
		input={configuration}
		onDemandOnly={(initialConfig?.type == 'static' || initialConfig?.type == 'evalv2') &&
			initialConfig?.onDemandOnly}
		bind:value={resolvedConfig}
		exportValueFunction
	/>
{/if}
