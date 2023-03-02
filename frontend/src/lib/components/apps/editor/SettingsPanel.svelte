<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import GridPanel from './GridPanel.svelte'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	const { selectedComponent, app } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#if $app.grid}
	<GridPanel parent={undefined} bind:gridItems={$app.grid} />
{/if}

{#if $app.subgrids}
	{#each Object.keys($app.subgrids ?? {}) as key (key)}
		<GridPanel parent={key} bind:gridItems={$app.subgrids[key]} />
	{/each}
{/if}

{#each $app?.hiddenInlineScripts ?? [] as script, index (script.name)}
	{#if $selectedComponent === `bg_${index}`}
		<PanelSection title={`Background script inputs`}>
			{Object.keys(script)}
			<InputsSpecsEditor
				id={`bg_${index}`}
				shouldCapitalize={false}
				bind:inputSpecs={script.fields}
				userInputEnabled={false}
			/>
		</PanelSection>
	{/if}
{/each}
