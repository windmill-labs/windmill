<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import ComponentPanel from './settingsPanel/ComponentPanel.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'
	import TablePanel from './TablePanel.svelte'

	const { selectedComponent, app } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#each $app.grid as gridItem (gridItem?.data?.id)}
	{#if gridItem?.data?.id === $selectedComponent}
		<ComponentPanel parent={undefined} bind:component={gridItem.data} />
	{:else if gridItem?.data?.type === 'tablecomponent'}
		<TablePanel bind:component={gridItem.data} />
	{/if}
{/each}

{#if $app.subgrids}
	{#each Object.keys($app.subgrids ?? {}) as key (key)}
		{#each $app.subgrids[key] as gridItem (gridItem?.data?.id)}
			{#if gridItem?.data?.id === $selectedComponent}
				<ComponentPanel parent={key} bind:component={gridItem.data} />
			{:else if gridItem?.data?.type === 'tablecomponent'}
				<TablePanel bind:component={gridItem.data} />
			{/if}
		{/each}
	{/each}
{/if}

{#each $app?.hiddenInlineScripts ?? [] as script, index (script.name)}
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
