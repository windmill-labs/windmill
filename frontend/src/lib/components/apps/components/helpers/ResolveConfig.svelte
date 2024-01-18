<script lang="ts">
	import { InputValue } from '.'
	import type { RichConfiguration } from '../../types'

	export let id: string
	export let extraKey: string = ''
	export let key: string
	export let resolvedConfig: any | { type: 'oneOf'; configuration: any; selected: string }
	export let configuration: RichConfiguration
	export let initialConfig: RichConfiguration | undefined = undefined
	$: configuration?.type == 'oneOf' && handleSelected(configuration.selected)

	function handleSelected(selected: string) {
		if (resolvedConfig?.selected != undefined && resolvedConfig?.selected != selected) {
			resolvedConfig.selected = selected
		}
	}
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
	/>
{/if}
