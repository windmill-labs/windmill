<script lang="ts">
	import type { RichConfigurations } from '../../types'

	import InputsSpecEditor from './InputsSpecEditor.svelte'

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
			{#if inputSpecs[k].ctype == undefined}
				{@const meta = inputSpecsConfiguration?.[k]}
				<!-- {JSON.stringify(meta)} -->
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
					optionValuesKey={meta?.['optionValuesKey']}
					tooltip={meta?.['tooltip']}
					onlyStatic={meta?.['onlyStatic']}
				/>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
