<script lang="ts">
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { getContext, onMount } from 'svelte'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import {
		components as componentsRecord,
		presets as presetsRecord,
		COMPONENT_SETS,
		type AppComponent,
		type TypedComponent
	} from '../component'
	import ListItem from './ListItem.svelte'
	import { appComponentFromType, insertNewGridItem } from '../appUtils'
	import { push } from '$lib/history'
	import { ClearableInput } from '../../../common'
	import { workspaceStore } from '$lib/stores'
	import { getGroup, listGroups } from './groupUtils'
	import { LayoutDashboard } from 'lucide-svelte'

	const { app, selectedComponent, focusedGrid } = getContext<AppViewerContext>('AppViewerContext')

	const { history } = getContext<AppEditorContext>('AppEditorContext')

	let groups: Array<{
		name: string
		path: string
	}> = []

	async function fetchGroups() {
		if ($workspaceStore) {
			groups = await listGroups($workspaceStore)
		}
	}

	function addComponent(appComponentType: TypedComponent['type']): void {
		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem(
			$app,
			appComponentFromType(appComponentType) as (id: string) => AppComponent,
			$focusedGrid
		)

		$selectedComponent = [id]
		$app = $app
	}

	async function addGroup(group: { name: string; path: string }) {
		if (!$workspaceStore) return
		const res = await getGroup($workspaceStore, group.path)

		if (!res) return

		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem($app, (id) => ({ ...res.value.component, id }), $focusedGrid)

		const subgrids = res.value?.subgrids ?? {}
		const newSubgrids = Object.entries(subgrids).reduce((acc, [key, value]) => {
			const newKey = key.replace(res.value.component.id, id)
			return {
				...acc,
				[newKey]: value
			}
		}, {})

		const [key, value] = newSubgrids[0]

		// @ts-ignore
		$app.subgrids[key] = value

		$selectedComponent = [id]
		$app = $app
	}

	function addPresetComponent(appComponentType: string): void {
		const preset = presetsRecord[appComponentType]

		push(history, $app)

		$dirtyStore = true

		const id = insertNewGridItem(
			$app,
			appComponentFromType(preset.targetComponent, preset.configuration) as (
				id: string
			) => AppComponent,
			$focusedGrid
		)

		$selectedComponent = [id]
		$app = $app
	}

	let search = ''

	$: componentsFiltered = COMPONENT_SETS.map((set) => ({
		...set,
		components: set.components.filter((component) => {
			const name = componentsRecord[component].name.toLowerCase()
			return name.includes(search.toLowerCase())
		}),
		presets: set.presets?.filter((preset) => {
			const presetName = presetsRecord[preset].name.toLowerCase()
			return presetName.includes(search.toLowerCase())
		})
	}))

	onMount(() => {
		fetchGroups()
	})
</script>

<section class="p-2 sticky w-full z-10 top-0 bg-surface border-b">
	<ClearableInput bind:value={search} placeholder="Search components..." />
</section>

<div class="relative">
	{#if componentsFiltered.reduce((acc, { components }) => acc + components.length, 0) === 0}
		<div class="absolute left-0 top-0 w-full text-sm text-tertiary text-center py-6 px-2">
			No components found
		</div>
	{:else}
		<div>
			{#each componentsFiltered as { title, components, presets }, index (index)}
				{#if components.length}
					<div>
						<ListItem title={`${title} (${components.length})`}>
							<div class="flex flex-wrap gap-3 py-2">
								{#each components as item (item)}
									<div class="w-20">
										<button
											on:click={() => addComponent(item)}
											title={componentsRecord[item].name}
											class="transition-all border w-20 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
											justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
										>
											<svelte:component this={componentsRecord[item].icon} class="text-primary" />
										</button>
										<div class="text-xs text-center flex-wrap text-secondary mt-1">
											{componentsRecord[item].name}
										</div>
									</div>
								{/each}
								{#if presets}
									{#each presets as presetItem (presetItem)}
										<div class="w-20">
											<button
												on:click={() => addPresetComponent(presetItem)}
												title={presetsRecord[presetItem].name}
												class="transition-all border w-20 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
										justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
											>
												<svelte:component
													this={presetsRecord[presetItem].icon}
													class="text-secondary"
												/>
											</button>
											<div class="text-xs text-center flex-wrap text-secondary mt-1">
												{presetsRecord[presetItem].name}
											</div>
										</div>
									{/each}
								{/if}
							</div>
						</ListItem>
					</div>
				{/if}
			{/each}
			<ListItem title={'Groups'}>
				<div class="flex flex-wrap gap-3 py-2">
					{#if groups}
						{#each groups as group (group)}
							<div class="w-20">
								<button
									on:click={() => {
										addGroup(group)
									}}
									title={group.name}
									class="transition-all border w-20 shadow-sm h-16 p-2 flex flex-col gap-2 items-center
										justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
								>
									<LayoutDashboard class="text-secondary" />
								</button>
								<div class="text-xs text-center flex-wrap text-secondary mt-1">
									{group.name}
								</div>
							</div>
						{/each}
					{/if}
				</div>
			</ListItem>
		</div>
	{/if}
</div>
