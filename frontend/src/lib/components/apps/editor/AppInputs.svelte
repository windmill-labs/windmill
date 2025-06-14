<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { BG_PREFIX, allItems } from '../utils'
	import AppComponentInput from './AppComponentInput.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	let resourceOnly: boolean = $state(true)
</script>

<Alert type="info" title="Configurations">
	In order to properly configure the app, you need to fill in the inputs below.
</Alert>
<div class="mt-2 flex flex-row-reverse">
	<Toggle bind:checked={resourceOnly} options={{ right: 'Resource only' }} />
</div>
<div class="gap-4 flex flex-col pt-4">
	{#each allItems(app.grid, app.subgrids) as gridItem (gridItem.data.id)}
		{#if gridItem?.data?.type === 'tablecomponent'}
			<div>
				<AppComponentInput bind:component={gridItem.data} {resourceOnly} />
				<div class="ml-4 mt-4">
					{#each gridItem.data.actionButtons as actionButton, actionIndex (actionButton.id)}
						<AppComponentInput
							bind:component={gridItem.data.actionButtons[actionIndex].data}
							{resourceOnly}
						/>
					{/each}
				</div>
			</div>
		{:else if gridItem?.data?.type === 'aggridcomponent' || gridItem?.data?.type === 'aggridcomponentee' || gridItem?.data?.type === 'dbexplorercomponent' || gridItem?.data?.type === 'aggridinfinitecomponent' || gridItem?.data?.type === 'aggridinfinitecomponentee'}
			<div>
				<AppComponentInput bind:component={gridItem.data} {resourceOnly} />
				<div class="ml-4 mt-4">
					{#if Array.isArray(gridItem.data.actions)}
						{#each gridItem.data.actions as actionButton, actionIndex (actionButton.id)}
							<AppComponentInput
								bind:component={gridItem.data.actions[actionIndex].data}
								{resourceOnly}
							/>
						{/each}
					{/if}
				</div>
			</div>
		{:else}
			<AppComponentInput bind:component={gridItem.data} {resourceOnly} />
		{/if}
	{/each}
</div>

{#if app?.hiddenInlineScripts?.length > 0}
	<div class="font-bold text-lg">Background runnable inputs</div>
	<div class="gap-4 flex flex-col pt-4">
		{#each app?.hiddenInlineScripts ?? [] as script, index (script.name)}
			<div class="border p-2">
				<div class="text-sm font-bold">{script.name}</div>

				{#if resourceOnly && Object.keys(script.fields).filter((fieldKey) => {
						const fields = script.fields
						const field = fields[fieldKey]
						return field.fieldType === 'object' && field.format?.startsWith('resource-')
					}).length === 0}
					<span class="text-sm text-secondary">No resource input</span>
				{:else}
					<InputsSpecsEditor
						id={BG_PREFIX + index}
						shouldCapitalize={false}
						bind:inputSpecs={script.fields}
						userInputEnabled={false}
						{resourceOnly}
					/>
				{/if}
			</div>
		{/each}
	</div>
{/if}
