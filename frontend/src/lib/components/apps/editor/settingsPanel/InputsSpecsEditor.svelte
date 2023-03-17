<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import type { RichConfigurations } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	export let id: string
	export let inputSpecs: RichConfigurations
	export let userInputEnabled: boolean = false
	export let shouldCapitalize: boolean = true
	export let rowColumns = false
	export let resourceOnly = false
</script>

{#if inputSpecs}
	<div class="w-full flex flex-col gap-4">
		{#each Object.keys(inputSpecs) as k}
			{#if inputSpecs[k].ctype == undefined}
				<InputsSpecEditor
					key={k}
					bind:componentInput={inputSpecs[k]}
					{id}
					{userInputEnabled}
					{shouldCapitalize}
					{resourceOnly}
					hasRows={rowColumns}
				/>
			{/if}
		{/each}
	</div>
{:else}
	<div class="text-gray-500 text-sm">No inputs</div>
{/if}
