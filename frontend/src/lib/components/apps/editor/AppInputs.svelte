<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import AppComponentInput from './AppComponentInput.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	const { app, lazyGrid } = getContext<AppEditorContext>('AppEditorContext')
</script>

<Alert type="info" title="Configurations">
	In order to properly configure the app, you need to fill in the inputs below.
</Alert>
{#each $lazyGrid as gridItem (gridItem.data.id)}
	{#if gridItem.data.type === 'tablecomponent'}
		<div>
			<AppComponentInput bind:component={gridItem.data} />
			<div class="ml-4 mt-4">
				{#each gridItem.data.actionButtons as actionButton (actionButton.id)}
					<AppComponentInput bind:component={actionButton.data} />
				{/each}
			</div>
		</div>
	{:else}
		<AppComponentInput bind:component={gridItem.data} />
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
		/>
	{/each}
</div>
