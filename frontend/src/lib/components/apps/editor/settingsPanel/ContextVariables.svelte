<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import { dfs, findGridItem } from '../appUtils'

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

	function addContextVariable(label: string, description: string): void {
		if (!contextVariables.some((variable) => variable.label === label)) {
			contextVariables.push({ label, description })
		}
	}

	if (tables.includes(type)) {
		addContextVariable(
			'row',
			'The current row of a table. Row is an object with keys index and value.'
		)
	} else if (type === 's3fileinputcomponent' || type === 'fileinputcomponent') {
		addContextVariable('file', 'The current file being processed.')
	} else if (type === 'containercomponent') {
		addContextVariable('group', 'The group name of the container.')
	} else if (type === 'listcomponent') {
		addContextVariable(
			'iter',
			'The current iteration of the list. Iter is an object with keys index and value.'
		)
	}

	function addGridItemContext(parentId: string) {
		if (!parentId) return
		const gridItem = findGridItem(app, parentId)
		addParentContextVariable(gridItem)
	}

	function processParents(parents: string[]) {
		parents.forEach((parentId) => {
			if (parentId) {
				const parsedParentId = parentId?.includes('-') ? parentId.split('-')[0] : parentId
				addGridItemContext(parsedParentId)
			}
		})
	}

	function findParentsContextVariables(id: string): void {
		if (!id) return
		const allParents = dfs(app.grid, id, app.subgrids ?? {})

		if (!allParents) return

		// Remove last element as it is the current component
		allParents.pop()

		processParents(allParents)
	}

	findParentsContextVariables(id)

	function addParentContextVariable(parent: GridItem | undefined) {
		if (parent?.data?.type === 'containercomponent') {
			addContextVariable('group', 'The group name of the container.')
		} else if (parent?.data?.type === 'listcomponent') {
			addContextVariable(
				'iter',
				'The current iteration of the list. Iter is an object with keys index and value.'
			)
		} else if (parent?.data?.type && tables.includes(parent?.data?.type)) {
			addContextVariable(
				'row',
				'The current row of a table. Row is an object with keys index and value.'
			)
		}
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
