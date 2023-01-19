<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { classNames } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import { getAllScriptNames } from '../../utils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import { getAppScripts } from './utils'

	export let selectedScriptComponentId: string | undefined = undefined
	const { app, selectedComponent, lazyGrid } = getContext<AppEditorContext>('AppEditorContext')

	function selectInlineScript(id: string) {
		selectedScriptComponentId = id
		if (!id.startsWith('unused-') || !id.startsWith('bg_')) {
			$selectedComponent = selectedScriptComponentId
		}
	}

	$: runnables = getAppScripts($lazyGrid)

	// When seleced component changes, update selectedScriptComponentId
	$: {
		if (selectedComponent) {
			selectedScriptComponentId = $selectedComponent
		}
	}

	function createBackgroundScript() {
		let index = 0
		let newScriptPath = `Background Script ${index}`

		const names = getAllScriptNames($app)

		// Find a name that is not used by any other inline script
		while (names.includes(newScriptPath)) {
			newScriptPath = `Background Script ${++index}`
		}

		if (!$app.hiddenInlineScripts) {
			$app.hiddenInlineScripts = []
		}

		$app.hiddenInlineScripts.push({
			name: newScriptPath,
			inlineScript: undefined,
			fields: {}
		})
		$app.hiddenInlineScripts = $app.hiddenInlineScripts
	}
</script>

<div class="min-h-full flex flex-col gap-4">
	<PanelSection title="Inline scripts" smallPadding>
		<div class="flex flex-col gap-2 w-full">
			{#if runnables.inline.length > 0}
				<div class="flex gap-2 flex-col ">
					{#each runnables.inline as { name, id }, index (index)}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							class="{classNames(
								'border flex  gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-sm cursor-pointer hover:bg-blue-50 hover:text-blue-400',
								selectedScriptComponentId === id ? 'border-blue-500 border' : ''
							)},"
							on:click={() => selectInlineScript(id)}
						>
							<span class="text-xs truncate">{name}</span>
							<div>
								<Badge color="dark-indigo">{id}</Badge>
							</div>
						</div>
					{/each}
				</div>
			{/if}

			{#if $app.unusedInlineScripts?.length > 0}
				<div class="flex gap-2 flex-col ">
					{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							class="{classNames(
								'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
								selectedScriptComponentId === `unused-${index}` ? 'bg-blue-100 text-blue-600' : ''
							)},"
							on:click={() => selectInlineScript(`unused-${index}`)}
						>
							<span class="text-xs truncate">{unusedInlineScript.name}</span>
							<Badge color="red">Detached</Badge>
						</div>
					{/each}
				</div>
			{/if}

			{#if runnables.inline.length == 0 && $app.unusedInlineScripts?.length == 0}
				<div class="text-sm text-gray-500">No inline scripts</div>
			{/if}
		</div>
	</PanelSection>

	<PanelSection title="Imported scripts" smallPadding>
		<div class="flex flex-col gap-2 w-full">
			{#if runnables.imported.length > 0}
				<div class="flex gap-2 flex-col ">
					{#each runnables.imported as { name, id }, index (index)}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							class="{classNames(
								'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
								selectedScriptComponentId === id ? 'bg-blue-100 text-blue-600' : ''
							)},"
							on:click={() => selectInlineScript(id)}
						>
							<span class="text-xs truncate">{name}</span>
							<Badge color="dark-indigo">{id}</Badge>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-sm text-gray-500">No items</div>
			{/if}
		</div>
	</PanelSection>

	<PanelSection title="Background scripts" smallPadding>
		<svelte:fragment slot="action">
			<Button size="xs" color="dark" startIcon={{ icon: faPlus }} on:click={createBackgroundScript}>
				Add
			</Button>
			<Tooltip>
				Background scripts are triggered upon global refresh or when their input changes. The result
				of a background script can be shared among many components.
			</Tooltip>
		</svelte:fragment>
		<div class="flex flex-col gap-2 w-full">
			{#if $app.hiddenInlineScripts?.length > 0}
				<div class="flex gap-2 flex-col ">
					{#each $app.hiddenInlineScripts as { name }, index (index)}
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div
							class="{classNames(
								'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
								selectedScriptComponentId === `bg_${index}` ? 'bg-blue-100 text-blue-600' : ''
							)},"
							on:click={() => selectInlineScript(`bg_${index}`)}
						>
							<span class="text-xs truncate">{name}</span>
							<Badge color="yellow">Background</Badge>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-sm text-gray-500">No items</div>
			{/if}
		</div>
	</PanelSection>
</div>
