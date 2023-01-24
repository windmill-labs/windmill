<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import AppComponentInput from './AppComponentInput.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	const { app, lazyGrid } = getContext<AppEditorContext>('AppEditorContext')

	let resourceOnly: boolean = true
</script>

<Alert type="info" title="Configurations">
	In order to properly configure the app, you need to fill in the inputs below.
</Alert>
<Toggle bind:checked={resourceOnly} options={{ right: 'Resource only' }} />
{#each $lazyGrid as gridItem (gridItem.data.id)}
	{#if gridItem.data.type === 'tablecomponent'}
		<div>
			<AppComponentInput bind:component={gridItem.data} {resourceOnly} />
			<div class="ml-4 mt-4">
				{#each gridItem.data.actionButtons as actionButton (actionButton.id)}
					<AppComponentInput bind:component={actionButton.data} {resourceOnly} />
				{/each}
			</div>
		</div>
	{:else}
		<AppComponentInput bind:component={gridItem.data} {resourceOnly} />
	{/if}
{/each}
<span class="font-bold text-sm">Background script inputs</span>
<div class="gap-4 flex flex-col">
	{#each $app?.hiddenInlineScripts ?? [] as script, index (script.name)}
		<span class="text-sm">{script.name}</span>
		<InputsSpecsEditor
			id={`bg_${index}`}
			shouldCapitalize={false}
			bind:inputSpecs={script.fields}
			userInputEnabled={false}
			{resourceOnly}
		/>
	{/each}
</div>
