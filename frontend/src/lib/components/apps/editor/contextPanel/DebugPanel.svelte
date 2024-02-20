<script lang="ts">
	import { getContext } from 'svelte'
	import { allItems } from '../../utils'
	import type { AppViewerContext } from '../../types'
	import Section from '$lib/components/Section.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { findGridItem } from '../appUtils'
	import { pluralize } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Trash } from 'lucide-svelte'
	import AutoDataTable from '$lib/components/table/AutoDataTable.svelte'

	const { app, initialized } = getContext<AppViewerContext>('AppViewerContext')

	$: unintitializedComponents = allItems($app.grid, $app.subgrids)
		.map((x) => x.id)
		.filter((x) => !$initialized.initializedComponents?.includes(x))
		.sort()

	$: subgridsErrors = Object.keys($app.subgrids ?? {})
		.map((x) => {
			const parentId = x.split('-')[0]
			const parent = findGridItem($app, parentId)

			if (parent === undefined) {
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
</script>

<div class="flex flex-col gap-8">
	{#if unintitializedComponents.length > 0}
		<Section label="Uninitialized components">
			<div class="max-w-xl">
				<div class="text-sm mb-4">
					There are {pluralize(unintitializedComponents.length, 'uninitialized component')} in the app.
					Please contact Windmill support for assistance with the following information:
				</div>
				{#if unintitializedComponents.length > 0}
					<DataTable>
						<Head>
							<tr>
								<Cell head first>Component Id</Cell>
								<Cell head>Type</Cell>
								<Cell head last>Status</Cell>
							</tr>
						</Head>
						<tbody class="divide-y">
							{#each unintitializedComponents as c}
								<tr>
									<Cell first>{c}</Cell>
									<Cell>
										<Badge color="blue">
											{findGridItem($app, c)?.data?.type || 'Unknown'}
										</Badge>
									</Cell>
									<Cell last>
										<Badge color="red">Uninitialized</Badge>
									</Cell>
								</tr>
							{/each}
						</tbody>
					</DataTable>
				{/if}
			</div>
		</Section>
		<Section label="Subgrids errors">
			<div class="max-w-xl">
				<div class="text-sm mb-4">
					There are
					{pluralize(subgridsErrors.length, 'subgrid')} with errors in the app.
				</div>
				{#if subgridsErrors.length > 0}
					<DataTable>
						<Head>
							<tr>
								<Cell head first>Subgrid Id</Cell>
								<Cell head>Error</Cell>
								<Cell head last>Action</Cell>
							</tr>
						</Head>
						<tbody class="divide-y">
							{#each subgridsErrors ?? [] as s}
								<tr>
									<Cell first>{s?.subGridId}</Cell>
									<Cell>
										<Badge color="red">
											{s?.error}
										</Badge>
									</Cell>
									<Cell last>
										<Button
											color="light"
											startIcon={{
												icon: Trash
											}}
											size="xs2"
											on:click={() => {}}
										>
											Remove subgrid
										</Button>
									</Cell>
								</tr>
							{/each}
						</tbody>
					</DataTable>
				{/if}
			</div>
		</Section>
	{/if}
</div>

<AutoDataTable
	objects={[
		{ a: 1, b: 2, c: 3 },
		{ a: 4, b: 5, c: 6 }
	]}
/>
