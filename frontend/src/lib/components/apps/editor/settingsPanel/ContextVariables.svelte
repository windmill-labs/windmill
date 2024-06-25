<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import { findGridItem, findGridItemParentGrid } from '../appUtils'

	export let type: string
	export let id: string

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	const tables = [
		'aggridcomponent',
		'aggridcomponentee',
		'dbexplorercomponent',
		'aggridinfinitecomponent',
		'aggridinfinitecomponentee'
	]

	const contextVariables: {
		label: string
		description: string
	}[] = []

	if (tables.includes(type)) {
		contextVariables.push({
			label: 'row',
			description: 'The current row of a table. Row is an object with keys index and value.'
		})
	} else if (type === 's3fileinputcomponent' || type === 'fileinputcomponent') {
		contextVariables.push({
			label: 'file',
			description: 'The current file being processed.'
		})
	} else if (type === 'containercomponent') {
		contextVariables.push({
			label: 'group',
			description: 'The group name of the container.'
		})
	} else if (type === 'listcomponent') {
		contextVariables.push({
			label: 'iter',
			description: 'The current iteration of the list. Iter is an object with keys index and value.'
		})
	}

	function findParent(id: string): GridItem | undefined {
		if (!id) return

		if (id?.includes('_')) {
			// This is an action of a table
			const parentId = id.split('_')?.[0]

			if (!parentId) return

			return findGridItem($app, parentId)
		} else {
			const subgrid = findGridItemParentGrid($app, id)

			if (subgrid && subgrid.includes('-')) {
				const parentId = subgrid?.split('-')?.[0]

				if (!parentId) return

				return findGridItem($app, parentId)
			}
		}
	}

	const parent = findParent(id)

	if (parent?.data?.type === 'containercomponent') {
		contextVariables.push({
			label: 'group',
			description: 'The group name of the container.'
		})
	} else if (parent?.data?.type === 'listcomponent') {
		contextVariables.push({
			label: 'iter',
			description: 'The current iteration of the list. Iter is an object with keys index and value.'
		})
	} else if (parent?.data?.type && tables.includes(parent?.data?.type)) {
		contextVariables.push({
			label: 'row',
			description: 'The current row of a table. Row is an object with keys index and value.'
		})
	}
</script>

{#if contextVariables.length > 0}
	<div class="my-2">
		<div class="text-xs font-semibold flex flex-row gap-1 items-center">
			<div class="mb-1"> Context </div>
			<Tooltip small light>
				Context are variables available in any expressions of config and runnable inputs that are
				specific to this particular component. Hover over the variables to see their descriptions.
			</Tooltip>
		</div>
		<div class="flex flex-row gap-1">
			{#each contextVariables as contextVariable}
				<Popover>
					<svelte:fragment slot="text">
						{contextVariable.description}
					</svelte:fragment>

					<span class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border">
						{contextVariable.label}
					</span>
				</Popover>
			{/each}
		</div>
	</div>
{/if}
