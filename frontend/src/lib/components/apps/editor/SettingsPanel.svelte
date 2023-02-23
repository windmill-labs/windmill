<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'
	import SettingsPanelRec from './SettingsPanelRec.svelte'

	const { selectedComponent, app } = getContext<AppEditorContext>('AppEditorContext')
</script>

<SettingsPanelRec bind:gridItems={$app.grid} />

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
