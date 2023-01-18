<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'
	import TablePanel from './TablePanel.svelte'

	const { selectedComponent, lazyGrid, app } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#each $lazyGrid as gridItem (gridItem.data.id)}
	{#if gridItem.data.id === $selectedComponent}
		<ComponentPanel bind:component={gridItem.data} />
	{:else if gridItem.data.type === 'tablecomponent'}
		<TablePanel bind:component={gridItem.data} />
	{/if}
{/each}

{#each $app.hiddenInlineScripts as script, index (script.name)}
	{#if $selectedComponent === `bg_${index}`}
		<PanelSection title={`Background script inputs`}>
			<InputsSpecsEditor
				id={`bg_${index}`}
				shouldCapitalize={false}
				bind:inputSpecs={script.fields}
				userInputEnabled={false}
			/>
		</PanelSection>
	{/if}
{/each}
