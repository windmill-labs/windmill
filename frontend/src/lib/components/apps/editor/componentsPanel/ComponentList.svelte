<script lang="ts">
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import { getContext, tick, untrack } from 'svelte'
	import {
		components as componentsRecord,
		presets as presetsRecord,
		COMPONENT_SETS,
		type AppComponent,
		type TypedComponent,
		DEPRECATED_COMPONENTS,
		processDimension
	} from '../component'
	import ListItem from './ListItem.svelte'
	import {
		appComponentFromType,
		copyComponent,
		insertNewGridItem,
		setUpTopBarComponentContent
	} from '../appUtils'
	import { push } from '$lib/history'
	import { ClearableInput, Drawer, DrawerContent } from '../../../common'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { getGroup, listGroups } from './groupUtils'
	import { LayoutDashboard, Plus } from 'lucide-svelte'
	import { ResourceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import ComponentsList from './CustomComponentsList.svelte'
	import Popover from '$lib/components/Popover.svelte'

	const { app, selectedComponent, focusedGrid } = getContext<AppViewerContext>('AppViewerContext')

	const { history, dndItem, yTop } = getContext<AppEditorContext>('AppEditorContext')

	let groups: Array<{
		name: string
		path: string
	}> = $state([])

	let customComponents: Array<{
		name: string
		path: string
	}> = $state([])

	async function fetchGroups() {
		groups = await listGroups($workspaceStore ?? '')
	}

	async function fetchCustomComponents() {
		customComponents = await ResourceService.listResourceNames({
			workspace: $workspaceStore ?? '',
			name: 'app_custom'
		})
	}

	function addComponent(appComponentType: TypedComponent['type']): string {
		push(history, app)

		const id = insertNewGridItem(
			app,
			appComponentFromType(appComponentType) as (id: string) => AppComponent,
			$focusedGrid
		)

		$selectedComponent = [id]
		// app = app
		return id
	}

	async function addGroup(group: { name: string; path: string }) {
		if (!$workspaceStore) return
		const res = await getGroup($workspaceStore, group.path)

		if (!res) return

		push(history, app)

		const id = copyComponent(app, res.value.item, $focusedGrid, res.value.subgrids, [])

		if (id) {
			$selectedComponent = [id]
			// app = app
		}
	}

	async function addNewGroup() {
		push(history, app)

		const id = insertNewGridItem(
			app,
			appComponentFromType('containercomponent', undefined, { groupFields: {} }) as (
				id: string
			) => AppComponent,
			$focusedGrid
		)

		if (id) {
			$selectedComponent = [id]
			// app = app
		}
	}

	async function addCustomComponent(cc: { name: string; path: string }) {
		if (!$workspaceStore) return
		let res: any = undefined
		try {
			res = await ResourceService.getResourceValue({
				workspace: $workspaceStore ?? '',
				path: cc.path
			})
		} catch (e) {
			sendUserToast(`Custom Component not found at ${cc.path}`)
			return
		}

		if (!res) return

		push(history, app)

		const id = insertNewGridItem(
			app,
			appComponentFromType('customcomponent', undefined, {
				customComponent: {
					name: cc.name.replace(/-/g, '_').replace(/\s/g, '_'),
					additionalLibs: {
						reactVersion: '18.2.0'
					}
				}
			}) as (id: string) => AppComponent,
			$focusedGrid
		)

		if (id) {
			$selectedComponent = [id]
			// app = app
		}
	}

	function addPresetComponent(appComponentType: string): void {
		const preset = presetsRecord[appComponentType]

		push(history, app)

		const id = insertNewGridItem(
			app,
			appComponentFromType(preset.targetComponent, preset.configuration, undefined) as (
				id: string
			) => AppComponent,
			$focusedGrid,
			undefined,
			undefined,
			{ x: 0, y: 0 },
			{
				3: processDimension(preset.dims, 3),
				12: processDimension(preset.dims, 12)
			}
		)

		$selectedComponent = [id]

		if (appComponentType === 'topbarcomponent') {
			setUpTopBarComponentContent(id, app)
		}

		// app = app
	}

	let search = $state('')

	let componentsFiltered = $derived(
		COMPONENT_SETS.map((set) => ({
			...set,
			components: set.components?.filter((component) => {
				const name = componentsRecord[component].name.toLowerCase()
				return name.includes(search.toLowerCase().trim())
			}),
			presets: set.presets?.filter((preset) => {
				const presetName = presetsRecord[preset].name.toLowerCase()
				return presetName.includes(search.toLowerCase().trim())
			})
		}))
	)

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				fetchGroups()
				fetchCustomComponents()
			})
		}
	})

	let dndTimeout: NodeJS.Timeout | undefined = $state(undefined)

	let ccDrawer: Drawer | undefined = $state()
</script>

<Drawer bind:this={ccDrawer}>
	<DrawerContent
		title="Custom Components"
		on:close={ccDrawer.closeDrawer}
		documentationLink="https://www.windmill.dev/docs/apps/react_components"
	>
		<ComponentsList on:reload={fetchCustomComponents} />
	</DrawerContent>
</Drawer>

<section class="p-2 sticky w-full z-10 top-0 bg-surface">
	<ClearableInput bind:value={search} placeholder="Search components..." />
</section>

<div class="relative" id="app-editor-component-list">
	{#if componentsFiltered.reduce((acc, { components, presets }) => acc + components.length + (Array.isArray(presets) ? presets.length : 0), 0) === 0}
		<div class="absolute left-0 top-0 w-full text-sm text-tertiary text-center py-6 px-2">
			No components found
		</div>
	{:else}
		<div>
			{#each componentsFiltered as { title, components, presets }, index (index)}
				{#if components.length || presets?.length}
					<div>
						<ListItem title={`${title}`} subtitle={`(${components.length})`}>
							<div class="flex flex-wrap gap-3 py-2">
								{#each components as item (item)}
									{@const SvelteComponent = componentsRecord[item].icon}
									<div class="w-[64px] relative">
										{#if DEPRECATED_COMPONENTS[item]}
											<div
												class="absolute -top-2 -right-2 bg-gray-100 text-gray-900 dark:bg-gray-800 dark:text-gray-100 rounded-md py-0.5 px-1 flex flex-row gap-1 items-center"
											>
												<Popover>
													{#snippet text()}
														<div>
															{DEPRECATED_COMPONENTS[item]}
														</div>
													{/snippet}
													<div class="font-normal text-2xs"> Deprecated </div>
												</Popover>
											</div>
										{/if}
										<button
											id={item}
											onpointerdown={async (e) => {
												const id = addComponent(item)
												dndTimeout && clearTimeout(dndTimeout)
												dndTimeout = setTimeout(async () => {
													await tick()
													$dndItem[id]?.(e.clientX, e.clientY, $yTop)
												}, 75)
												window.addEventListener('pointerup', (e) => {
													dndTimeout && clearTimeout(dndTimeout)
													dndTimeout = undefined
												})
											}}
											title={componentsRecord[item].name}
											class="cursor-move transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
											justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
										>
											<SvelteComponent class="text-primary" />
										</button>
										<div class="text-xs text-center flex-wrap text-secondary mt-1">
											{componentsRecord[item].name}
										</div>
									</div>
								{/each}
								{#if presets}
									{#each presets as presetItem (presetItem)}
										{@const SvelteComponent_1 = presetsRecord[presetItem].icon}
										<div class="w-[64px]">
											<button
												onclick={() => addPresetComponent(presetItem)}
												title={presetsRecord[presetItem].name}
												class="transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
										justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
											>
												<SvelteComponent_1 class="text-secondary" />
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
						{#each groups as group (group.path)}
							<div class="w-[64px]">
								<button
									onclick={() => {
										addGroup(group)
									}}
									title={group.name}
									class="transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
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
					<div class="w-[64px]">
						<button
							onclick={() => {
								addNewGroup()
							}}
							title=""
							class="transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
								justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
						>
							<Plus class="text-secondary" />
						</button>
						<div class="text-xs text-center flex-wrap text-secondary mt-1"> Add new </div>
					</div>
				</div>
			</ListItem>
			<ListItem title={'Custom Components'} tooltip={'Create components in React or Vanilla JS'}>
				<div class="flex flex-wrap gap-3 py-2">
					{#if customComponents}
						{#each customComponents as cc (cc.path)}
							<div class="w-[64px]">
								<button
									onclick={() => {
										addCustomComponent(cc)
									}}
									title={cc.name}
									class="transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
										justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
								>
									<LayoutDashboard class="text-secondary" />
								</button>
								<div class="text-xs text-center flex-wrap text-secondary mt-1">
									{cc.name}
								</div>
							</div>
						{/each}
					{/if}
					<div class="w-[64px]">
						<button
							onclick={() => {
								if (!$enterpriseLicense) {
									sendUserToast('Custom components are only available on the EE', true)
								} else {
									ccDrawer?.openDrawer()
								}
							}}
							title=""
							class="transition-all border w-[64px] shadow-sm h-16 p-2 flex flex-col gap-2 items-center
								justify-center bg-surface rounded-md hover:bg-blue-50 dark:hover:bg-blue-900 duration-200 hover:border-blue-500"
						>
							<Plus class="text-secondary" />
						</button>
						<div class="text-xs text-center flex-wrap text-secondary mt-1"> Add new </div>
					</div>
				</div>
			</ListItem>
		</div>
	{/if}
</div>
