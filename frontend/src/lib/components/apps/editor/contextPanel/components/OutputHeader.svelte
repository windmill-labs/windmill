<script lang="ts">
	import type { AppViewerContext, ContextPanelContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronUp, Pointer } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { allsubIds, findGridItem } from '../../appUtils'
	import IdEditor from './IdEditor.svelte'
	import type { AppComponent } from '../../component'
	import type { Runnable } from '$lib/components/apps/inputType'

	export let id: string
	export let name: string
	export let first: boolean = false
	export let nested: boolean = false
	export let color: 'blue' | 'indigo' = 'indigo'
	export let selectable: boolean = true
	export let renamable: boolean = true
	export let disabled: boolean = false

	const { manuallyOpened, search, hasResult } = getContext<ContextPanelContext>('ContextPanel')

	const { selectedComponent, app, hoverStore, allIdsInPath, connectingInput, worldStore } =
		getContext<AppViewerContext>('AppViewerContext')

	$: subids = $search != '' ? allsubIds($app, id) : []
	$: inSearch =
		$search != '' &&
		($hasResult[id] ||
			Object.entries($hasResult).some(([key, value]) => value && subids.includes(key)))
	$: open =
		$allIdsInPath.includes(id) || id == $selectedComponent?.[0] || $manuallyOpened[id] || inSearch

	const hoverColor = {
		blue: 'hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-frost-900 dark:hover:text-frost-100',
		indigo:
			'hover:bg-indigo-100 hover:text-indigo-500 dark:hover:bg-frost-900 dark:hover:text-indigo-300'
	}

	const openBackground = {
		blue: 'bg-blue-50 dark:bg-frost-800',
		indigo: 'bg-indigo-50 dark:bg-indigo-800'
	}

	const manuallyOpenColor = {
		blue: 'text-primary bg-gray-300 rounded-sm',
		indigo: 'text-primary bg-gray-300 rounded-sm'
	}

	const idClass = {
		blue: 'bg-blue-500 text-white',
		indigo: 'bg-indigo-500 text-white'
	}

	function renameId(newId: string): void {
		const item = findGridItem($app, id)

		if (!item) {
			return
		}
		item.data.id = newId
		item.id = newId

		const oldSubgrids = Object.keys($app.subgrids ?? {}).filter((subgrid) =>
			subgrid.startsWith(id + '-')
		)

		oldSubgrids.forEach((subgrid) => {
			if ($app.subgrids) {
				$app.subgrids[subgrid.replace(id, newId)] = $app.subgrids[subgrid]
				delete $app.subgrids[subgrid]
			}
		})

		function propagateRename(from: string, to: string) {
			allItems($app.grid, $app.subgrids).forEach((item) => {
				renameComponent(from, to, item.data)
			})

			$app.hiddenInlineScripts?.forEach((x) => {
				processRunnable(from, to, x)
			})
		}
		propagateRename(id, newId)
		if (item?.data.type == 'tablecomponent') {
			for (let c of item.data.actionButtons) {
				let old = c.id
				c.id = c.id.replace(id + '_', newId + '_')
				propagateRename(old, c.id)
			}
		}

		if (item?.data.type === 'menucomponent') {
			for (let c of item.data.menuItems) {
				let old = c.id
				c.id = c.id.replace(id + '_', newId + '_')
				propagateRename(old, c.id)
			}
		}

		$app = $app
		$selectedComponent = [newId]

		delete $worldStore.outputsById[id]
	}

	function renameComponent(from: string, to: string, data: AppComponent) {
		if (data.type == 'tablecomponent') {
			for (let c of data.actionButtons) {
				renameComponent(from, to, c)
			}
		}

		if (data.type === 'menucomponent') {
			for (let c of data.menuItems) {
				renameComponent(from, to, c)
			}
		}

		let componentInput = data.componentInput
		if (componentInput?.type == 'connected') {
			if (componentInput.connection?.componentId === from) {
				componentInput.connection.componentId = to
			}
		} else if (componentInput?.type == 'runnable') {
			processRunnable(from, to, componentInput.runnable)
			Object.values(componentInput.fields).forEach((field) => {
				if (field.type == 'connected') {
					if (field.connection?.componentId === from) {
						field.connection.componentId = to
					}
				}
			})
		}

		Object.values(data.configuration ?? {}).forEach((config) => {
			if (config.type === 'connected') {
				if (config.connection?.componentId === from) {
					config.connection.componentId = to
				}
			} else if (config.type == 'oneOf') {
				Object.values(config.configuration ?? {}).forEach((choices) => {
					Object.values(choices).forEach((c) => {
						if (c.type === 'connected') {
							if (c.connection?.componentId === id) {
								c.connection.componentId = to
							}
						}
					})
				})
			}
		})
	}

	function processRunnable(from: string, to: string, runnable: Runnable) {
		if (
			runnable?.type === 'runnableByName' &&
			runnable?.inlineScript?.refreshOn?.find((x) => x.id === from)
		) {
			runnable.inlineScript.refreshOn = runnable.inlineScript.refreshOn.map((x) => {
				if (x.id === from) {
					return {
						id: to,
						key: x.key
					}
				}
				return x
			})
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class={$search == '' || inSearch ? '' : 'invisible h-0 overflow-hidden'}>
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		on:mouseenter|stopPropagation={() => {
			if (id !== $hoverStore) {
				$hoverStore = id
			}
		}}
		on:mouseleave|stopPropagation={() => {
			if ($hoverStore !== undefined) {
				$hoverStore = undefined
			}
		}}
		class={classNames(
			'flex items-center justify-between p-1 cursor-pointer gap-1 truncate',
			hoverColor[color],
			$selectedComponent?.includes(id)
				? openBackground[color]
				: $connectingInput.hoveredComponent === id
				? 'bg-orange-300 '
				: 'bg-surface-secondary',
			first ? 'border-t' : '',
			nested ? 'border-l' : '',
			'transition-all'
		)}
		on:click={() => {
			if (!disabled) {
				$manuallyOpened[id] = $manuallyOpened[id] != undefined ? !$manuallyOpened[id] : true
			}
		}}
		id={`output-${id}`}
	>
		<div class="flex">
			<button
				disabled={!(selectable && !$selectedComponent?.includes(id)) || $connectingInput?.opened}
				title="Select component"
				on:click|stopPropagation={() => ($selectedComponent = [id])}
				class="flex items-center ml-0.5 rounded-sm bg-surface-selected hover:text-primary text-tertiary"
			>
				<div
					class={classNames(
						'text-2xs  font-bold px-2 py-0.5 rounded-sm',
						$selectedComponent?.includes(id) ? idClass[color] : ''
					)}
				>
					{id}
				</div>
				{#if selectable && !$selectedComponent?.includes(id)}
					<div class="px-1">
						<Pointer size={14} />
					</div>
				{/if}
			</button>
			{#if selectable && renamable && $selectedComponent?.includes(id)}
				<div class="h-3">
					<IdEditor
						{id}
						on:selected={() => ($selectedComponent = [id])}
						on:change={({ detail }) => renameId(detail)}
					/></div
				>
			{/if}
		</div>
		<div class="text-2xs font-bold flex flex-row gap-2 items-center truncate">
			{name}
			<div class={classNames('bg-surface-secondary rounded-sm')}>
				{#if !open}
					<ChevronDown size={14} color="gray" />
				{:else if $manuallyOpened[id]}
					<ChevronUp size={14} class={manuallyOpenColor[color]} strokeWidth={4} />
				{:else}
					<ChevronUp size={14} color="gray" />
				{/if}
			</div>
		</div>
	</div>
	<div
		class="border-b {open
			? 'h-full'
			: 'h-0 overflow-hidden invisible'} {$connectingInput.hoveredComponent === id &&
		!$selectedComponent?.includes(id)
			? '  bg-orange-100/40'
			: ''}"
	>
		<div class={classNames(nested ? 'border-l ml-2' : '', open ? 'border-t' : '')}>
			<slot />
		</div>
	</div>
</div>
