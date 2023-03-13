<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import GridPanel from './GridPanel.svelte'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	const { selectedComponent, app, stateId } = getContext<AppViewerContext>('AppViewerContext')
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
		<div class="min-h-full flex flex-col divide-y">
			<PanelSection title={`Configuration`}>
				<Toggle
					bind:checked={script.autoRefresh}
					options={{ right: 'Run on start and app refresh' }}
				/>
			</PanelSection>

			{#if Object.keys(script.fields).length > 0}
				<PanelSection title={`Inputs`}>
					{#key $stateId}
						<InputsSpecsEditor
							id={`bg_${index}`}
							shouldCapitalize={false}
							bind:inputSpecs={script.fields}
							userInputEnabled={false}
						/>
					{/key}
				</PanelSection>
			{/if}
			<div class="grow shrink" />
		</div>
	{/if}
{/each}
