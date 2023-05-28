<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import { Plus } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import Tooltip from '../../../Tooltip.svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { BG_PREFIX, getAllScriptNames } from '../../utils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import { getAppScripts } from './utils'

	const PREFIX = 'script-selector-' as const

	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')

	function selectScript(id: string) {
		$selectedComponentInEditor = id
		if (!id.startsWith('unused-') || !id.startsWith(BG_PREFIX)) {
			$selectedComponent = [$selectedComponentInEditor.split('_transformer')[0]]
		}
	}

	$: runnables = getAppScripts($app.grid, $app.subgrids)

	// When selected component changes, update selectedScriptComponentId
	$: handleSelectedComponent($selectedComponent)

	function handleSelectedComponent(selectedComponent: string[] | undefined) {
		if (
			selectedComponent != $selectedComponentInEditor &&
			!$selectedComponentInEditor?.includes('_transformer')
		) {
			$selectedComponentInEditor = selectedComponent?.[0]
		}
	}

	function createBackgroundScript() {
		for (const [index, script] of $app.hiddenInlineScripts.entries()) {
			if (script.hidden) {
				delete script.hidden
				$app.hiddenInlineScripts = $app.hiddenInlineScripts
				selectScript(BG_PREFIX + index)
				return
			}
		}
		let index = 0
		let newScriptPath = `Background Runnable ${index}`

		const names = getAllScriptNames($app)

		// Find a name that is not used by any other inline script
		while (names.includes(newScriptPath)) {
			newScriptPath = `Background Runnable ${++index}`
		}

		if (!$app.hiddenInlineScripts) {
			$app.hiddenInlineScripts = []
		}

		$app.hiddenInlineScripts.push({
			name: newScriptPath,
			inlineScript: undefined,
			autoRefresh: true,
			type: 'runnableByName',
			fields: {},
			recomputeIds: undefined
		})
		$app.hiddenInlineScripts = $app.hiddenInlineScripts
		selectScript(`${BG_PREFIX}${$app.hiddenInlineScripts.length - 1}`)
	}
</script>

<PanelSection title="Runnables">
	<div class="w-full flex flex-col gap-6 py-1">
		<div>
			<div class="flex flex-col gap-2 w-full">
				{#if runnables.inline.length > 0}
					<div class="flex gap-1 flex-col">
						{#each runnables.inline as { name, id, transformer }, index (index)}
							<button
								id={PREFIX + id}
								class="panel-item
				{$selectedComponentInEditor === id ? 'border-blue-500 bg-blue-100' : 'hover:bg-blue-50'}"
								on:click={() => selectScript(id)}
							>
								<span class="text-2xs truncate">{name}</span>
								<div>
									<Badge color="indigo">{id}</Badge>
								</div>
							</button>
							{#if transformer}
								<div class="w-full pl-4">
									<button
										id={PREFIX + id + '_transformer'}
										class="border flex gap-1 truncate font-normal justify-between w-full items-center px-2 py-0.5 rounded-sm duration-200;
			{$selectedComponentInEditor === id + '_transformer'
											? 'border-blue-500 bg-blue-100'
											: 'hover:bg-blue-50'}"
										on:click={() => selectScript(id + '_transformer')}
									>
										<span class="text-2xs truncate">Transformer</span>
									</button>
								</div>
							{/if}
						{/each}
					</div>
				{/if}
				{#each runnables.imported as { name, id, transformer }, index (index)}
					<button
						id={PREFIX + id}
						class="panel-item
						{$selectedComponentInEditor === id ? 'border-blue-500 bg-blue-100' : 'hover:bg-blue-50'}"
						on:click={() => selectScript(id)}
					>
						<span class="text-2xs truncate">{name}</span>
						<Badge color="indigo">{id}</Badge>
					</button>
					{#if transformer}
						<div class="w-full pl-4">
							<button
								id={PREFIX + id + '_transformer'}
								class="border flex gap-1 truncate font-normal justify-between w-full items-center px-2 py-0.5 rounded-sm duration-200;
{$selectedComponentInEditor === id + '_transformer'
									? 'border-blue-500 bg-blue-100'
									: 'hover:bg-blue-50'}"
								on:click={() => selectScript(id + '_transformer')}
							>
								<span class="text-2xs truncate">Transformer</span>
							</button>
						</div>
					{/if}
				{/each}

				{#if $app.unusedInlineScripts?.length > 0}
					<div class="flex gap-1 flex-col">
						{#each $app.unusedInlineScripts as unusedInlineScript, index (index)}
							{@const id = `unused-${index}`}
							<button
								id={PREFIX + id}
								class="panel-item
								{$selectedComponentInEditor === id ? 'border-blue-500 bg-blue-100' : 'hover:bg-blue-50'}"
								on:click={() => selectScript(id)}
							>
								<span class="text-2xs truncate">{unusedInlineScript.name}</span>
								<Badge color="red">Detached</Badge>
							</button>
						{/each}
					</div>
				{/if}
				{#if runnables.inline.length == 0 && $app.unusedInlineScripts?.length == 0 && runnables.imported.length == 0}
					<div class="text-xs text-gray-500">No scripts/flows</div>
				{/if}
			</div>
		</div>

		<div>
			<div class="w-full flex justify-between items-center mb-1">
				<div class="text-xs text-gray-600 font-semibold truncate">
					Background runnables
					<Tooltip
						class="mb-0.5"
						documentationLink="https://docs.windmill.dev/docs/apps/app-runnable#background-runnable"
					>
						Background runnables can be triggered on app refresh or when their input changes. The
						result can be shared among many components.
					</Tooltip>
				</div>
				<Button
					size="xs"
					color="light"
					variant="border"
					btnClasses="!rounded-full !p-1"
					title="Create a new background runnable"
					aria-label="Create a new background runnable"
					on:click={createBackgroundScript}
				>
					<Plus size={14} class="text-gray-500" />
				</Button>
			</div>
			<div class="flex flex-col gap-1 w-full">
				{#if $app.hiddenInlineScripts?.length > 0}
					{#each $app.hiddenInlineScripts as { name, hidden }, index (index)}
						{#if !hidden}
							{@const id = BG_PREFIX + index}
							<button
								id={PREFIX + id}
								class="panel-item
								{$selectedComponentInEditor === id ? 'border-blue-500 bg-blue-100' : 'hover:bg-blue-50'}"
								on:click={() => selectScript(id)}
							>
								<span class="text-2xs truncate">{name}</span>
								<Badge color="indigo">{id}</Badge>
							</button>
						{/if}
					{/each}
				{:else}
					<div class="text-xs text-gray-500">No background runnable</div>
				{/if}
			</div>
		</div>
	</div>
</PanelSection>

<style lang="postcss">
	.panel-item {
		@apply border flex gap-1 truncate font-normal justify-between w-full items-center py-1 px-2 rounded-sm duration-200;
	}
</style>
