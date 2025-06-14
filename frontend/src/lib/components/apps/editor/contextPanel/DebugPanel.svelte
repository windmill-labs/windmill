<script lang="ts">
	import { getContext } from 'svelte'
	import { allItems } from '../../utils'
	import type { AppViewerContext } from '../../types'
	import Section from '$lib/components/Section.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { deleteGridItem, findGridItem, findGridItemParentGrid } from '../appUtils'
	import { pluralize } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Trash } from 'lucide-svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	const { app, initialized } = getContext<AppViewerContext>('AppViewerContext')

	let unintitializedComponents = $derived(
		allItems(app.grid, app.subgrids)
			.map((x) => x.id)
			.filter((x) => !$initialized.initializedComponents?.includes(x))
			.sort()
	)

	let subgridsErrors = $derived(
		Object.keys(app.subgrids ?? {})
			.map((x) => {
				const parentId = x.split('-')[0]
				const parent = findGridItem(app, parentId)
				const subgrid = x.replace(`${parentId}-`, '')
				if (subgrid == '-1') {
					return {
						subGridId: x,
						error: 'Invalid subgrid index -1 '
					}
				} else if (parent === undefined) {
					return {
						subGridId: x,
						error: 'Parent not found'
					}
				} else if (parent?.data?.numberOfSubgrids === undefined) {
					return {
						subGridId: x,
						error: 'Parent is not a container'
					}
				}
			})
			.filter(Boolean)
	)
</script>

<div class="flex flex-col gap-8" style="all:none;">
	{#if unintitializedComponents?.length === 0 && subgridsErrors?.length === 0}
		<Alert type="success" title="No issues found">
			The app has no subgrid errors or uninitialized components.
		</Alert>
	{:else}
		<Alert type="error" title="Issues found">
			The app has {unintitializedComponents.length} uninitialized components and {subgridsErrors.length}
			subgrid errors.
			<br />
			Please contact Windmill support for assistance.
		</Alert>
	{/if}
	{#if unintitializedComponents.length > 0}
		<Section label="Uninitialized components">
			<div class="max-w-xl">
				<div class="text-sm mb-4">
					There are {pluralize(unintitializedComponents.length, 'uninitialized component')} in the app.
				</div>

				<div class="grid grid-cols-4 border rounded-md overflow-hidden">
					<!-- Header -->
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Component Id</div
					>
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Type</div
					>
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Status</div
					>
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Action</div
					>

					<!-- Iterate over uninitializedComponents to display each component in the grid -->
					{#each unintitializedComponents as c}
						{@const item = findGridItem(app, c)}
						{#if !item}
							<div>Item {c} not found</div>
						{:else}
							<!-- Component Id -->
							<div class="text-xs flex items-center px-2 py-2">
								<Badge>
									{c}
								</Badge>
							</div>

							<div class="text-xs flex items-center px-2 py-2">
								<Badge color="blue">
									{item?.data?.type || 'Unknown'}
								</Badge>
							</div>

							<div class="text-xs flex items-center px-2 py-2">
								<Badge color="red">Uninitialized</Badge>
							</div>
							<div class="text-xs flex items-center px-2 py-2">
								<Button
									color="light"
									startIcon={{
										icon: Trash
									}}
									size="xs2"
									on:click={() => {
										let parent = findGridItemParentGrid(app, c)
										deleteGridItem(app, item.data, parent) // $app = $app
									}}
								>
									Remove
								</Button>
							</div>
						{/if}
					{/each}
				</div>
			</div>
		</Section>{/if}
	{#if subgridsErrors.length > 0}
		<Section label="Subgrids errors">
			<div class="max-w-xl">
				<div class="text-sm mb-4">
					There are
					{pluralize(subgridsErrors.length, 'subgrid')} with errors in the app.
				</div>

				<div class="grid grid-cols-3 border rounded-md overflow-hidden">
					<!-- Header -->
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Subgrid Id</div
					>
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Error</div
					>
					<div class="font-semibold bg-gray-100 dark:bg-gray-900 px-2 py-1 text-xs border-b"
						>Action</div
					>

					<!-- Iterate over uninitializedComponents to display each component in the grid -->
					{#each subgridsErrors as s}
						<!-- Component Id -->
						<div class="text-xs flex items-center px-2 py-2">
							<Badge>
								{s?.subGridId}
							</Badge>
						</div>

						<div class="text-xs flex items-center px-2 py-2">
							<Badge color="red">
								{s?.error}
							</Badge>
						</div>

						<div class="text-xs flex items-center px-2 py-2">
							<Button
								color="light"
								startIcon={{
									icon: Trash
								}}
								size="xs2"
								on:click={() => {
									if (app.subgrids && s) {
										delete app.subgrids[s.subGridId]
									}
								}}
							>
								Remove
							</Button>
						</div>
					{/each}
				</div>
			</div>
		</Section>
	{/if}
</div>
