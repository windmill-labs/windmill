<script lang="ts">
	import { Badge } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'

	export let selectedScriptComponentId: string | undefined = undefined

	const { app, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	function selectInlineScript(id: string) {
		selectedScriptComponentId = id
		$selectedComponent = id
	}

	$: runnablesByName = $app.grid.reduce((acc, gridComponent) => {
		const componentInput: AppInput = gridComponent.data.componentInput

		if (componentInput?.type === 'runnable') {
			if (componentInput.runnable?.type === 'runnableByName') {
				acc.push({
					name: componentInput.runnable.name,
					id: gridComponent.id
				})
			}
		}
		return acc
	}, [])

	$: runnablesByPath = $app.grid.reduce((acc, gridComponent) => {
		const componentInput: AppInput = gridComponent.data.componentInput

		if (componentInput?.type === 'runnable') {
			if (componentInput.runnable?.type === 'runnableByPath') {
				acc.push({
					name: componentInput.runnable.path,
					id: gridComponent.id
				})
			}
		}
		return acc
	}, [])

	// When seleced component changes, update selectedScriptComponentId
	$: {
		if (selectedComponent) {
			selectedScriptComponentId = $selectedComponent
		}
	}
</script>

<PanelSection title="Inline scripts" smallPadding>
	<div class="flex flex-col gap-2 w-full">
		{#if runnablesByName.length > 0}
			<div class="flex gap-2 flex-col ">
				{#each runnablesByName as { name, id }, index (index)}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class="{classNames(
							'border flex justify-between flex-row w-full items-center p-2 rounded-sm cursor-pointer hover:bg-blue-50 hover:text-blue-400',
							selectedScriptComponentId === id ? 'border-blue-500 border' : ''
						)},"
						on:click={() => selectInlineScript(id)}
					>
						<span class="text-xs">{name}</span>
						<Badge color="dark-indigo">{id}</Badge>
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-sm text-gray-500">No inline scripts</div>
		{/if}

		{#if Array.isArray($app.unusedInlineScripts) && $app.unusedInlineScripts.length > 0}
			<div class="flex gap-2 flex-col ">
				{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class="{classNames(
							'border flex justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
							selectedScriptComponentId === '' ? 'bg-blue-100 text-blue-600' : ''
						)},"
						on:click={() => selectInlineScript(unusedInlineScript.name)}
					>
						<span class="text-xs">{unusedInlineScript.name}</span>
						<Badge color="red">Unused</Badge>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</PanelSection>

<PanelSection title="Runnable by path" smallPadding>
	<div class="flex flex-col gap-2 w-full">
		{#if runnablesByPath.length > 0}
			<div class="flex gap-2 flex-col ">
				{#each runnablesByPath as { name, id }, index (index)}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class="{classNames(
							'border flex justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
							selectedScriptComponentId === id ? 'bg-blue-100 text-blue-600' : ''
						)},"
						on:click={() => selectInlineScript(id)}
					>
						<span class="text-xs">{name}</span>
						<Badge color="dark-indigo">{id}</Badge>
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-sm text-gray-500">No inline scripts</div>
		{/if}
	</div>
</PanelSection>
