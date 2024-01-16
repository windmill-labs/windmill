<script lang="ts">
	import { ArrowDown, ArrowUp, Download, EyeIcon, MoreVertical } from 'lucide-svelte'
	import Dropdown from '../DropdownV2.svelte'
	import Cell from './Cell.svelte'
	import DataTable from './DataTable.svelte'
	import Head from './Head.svelte'
	import Row from './Row.svelte'
	import { pluralize } from '$lib/utils'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let objects: Array<Record<string, any>> = []

	let currentPage = 1
	let perPage = 10
	let search: string = ''

	$: data = objects
		.filter((row) => {
			return Object.values(row).some((value) => {
				return value.toString().toLowerCase().includes(search.toLowerCase())
			})
		})
		.sort((a, b) => {
			if (activeSorting) {
				if (activeSorting.direction == 'asc') {
					return a[activeSorting.column] > b[activeSorting.column] ? 1 : -1
				} else {
					return a[activeSorting.column] < b[activeSorting.column] ? 1 : -1
				}
			} else {
				return 0
			}
		})
		.slice((currentPage - 1) * perPage, currentPage * perPage)
		.map((row) => {
			return Object.fromEntries(
				Object.entries(row).filter(([key, value]) => {
					return !hiddenColumns.includes(key)
				})
			)
		})

	let hiddenColumns = [] as Array<string>
	let activeSorting:
		| {
				column: string
				direction: 'asc' | 'desc'
		  }
		| undefined = undefined

	let selection = [] as Array<string>

	// Function to handle individual row checkbox change
	function handleCheckboxChange(rowId) {
		if (selection.includes(rowId)) {
			// Remove the id from the selection array
			selection = selection.filter((id) => id !== rowId)
		} else {
			// Add the id to the selection array
			selection = [...selection, rowId]
		}
	}

	// Function to handle select all checkbox change
	function handleSelectAllChange(event) {
		if (event.target.checked) {
			// Select all rows
			selection = data.map((row) => row.id)
		} else {
			// Deselect all rows
			selection = []
		}
	}

	let renderCount = 0

	const badgeColors = ['gray', 'blue', 'red', 'green', 'yellow', 'indigo']
</script>

<div class="flex flex-col divide-y gap-2 pt-4 mt-4">
	<div class="flex flex-row justify-between items-center">
		<div class="flex flex-row gap-2 items-center whitespace-nowrap w-full">
			<input bind:value={search} placeholder="Search..." class="h-8 !text-xs w-24" />
			{#if selection.length > 0}
				<span class="text-xs text-gray-500">
					{pluralize(selection?.length ?? 1, 'item') + ' selected'}
				</span>
			{/if}
		</div>
		<Dropdown
			items={() => {
				const actions = [
					{
						displayName: 'Download CSV',
						icon: Download,
						action: () => {
							const csv = objects
								.map((row) => {
									return Object.values(row).join(',')
								})
								.join('\n')

							const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
							const url = URL.createObjectURL(blob)
							const link = document.createElement('a')
							link.setAttribute('href', url)
							link.setAttribute('download', 'data.csv')
							link.style.visibility = 'hidden'
							document.body.appendChild(link)
							link.click()

							document.body.removeChild(link)
						}
					},
					{
						displayName: 'Download JSON',
						icon: Download,
						action: () => {
							const json = JSON.stringify(objects, null, 2)

							const blob = new Blob([json], { type: 'text/json;charset=utf-8;' })
							const url = URL.createObjectURL(blob)
							const link = document.createElement('a')
							link.setAttribute('href', url)
							link.setAttribute('download', 'data.json')
							link.style.visibility = 'hidden'
							document.body.appendChild(link)
							link.click()

							document.body.removeChild(link)
						}
					},
					{
						displayName: 'Restore hidden columns',
						icon: EyeIcon,
						action: () => {
							hiddenColumns = []
						}
					}
				]

				if (selection.length > 0) {
					actions.push({
						displayName: 'Delete selected',
						icon: EyeIcon,
						action: () => {
							selection = []
							renderCount++
						}
					})

					// Download selected as CSV
					actions.push({
						displayName: 'Download selected as CSV',
						icon: Download,
						action: () => {
							const csv = objects
								.filter((row) => selection.includes(JSON.stringify(row)))
								.map((row) => {
									return Object.values(row).join(',')
								})
								.join('\n')

							const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
							const url = URL.createObjectURL(blob)
							const link = document.createElement('a')
							link.setAttribute('href', url)
							link.setAttribute('download', 'data.csv')
							link.style.visibility = 'hidden'
							document.body.appendChild(link)
							link.click()

							document.body.removeChild(link)
						}
					})
				}

				return actions
			}}
		>
			<svelte:fragment slot="buttonReplacement">
				<MoreVertical
					size={8}
					class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
				/>
			</svelte:fragment>
		</Dropdown>
	</div>
	{#key renderCount}
		<DataTable
			size="sm"
			shouldHidePagination={false}
			paginated={true}
			{perPage}
			{currentPage}
			on:next={() => {
				currentPage += 1
			}}
			on:previous={() => {
				currentPage -= 1
			}}
			on:change={(event) => {
				currentPage = event.detail
			}}
			showNext={currentPage * perPage < objects.length}
		>
			<Head>
				<tr>
					<Cell head first={true} last={false} class="w-6">
						<input type="checkbox" class="!w-4 !h-4" on:change={handleSelectAllChange} />
					</Cell>
					{#each Object.keys(data[0] ?? {}) ?? [] as key, index}
						<Cell head first={index == 0} last={index == Object.keys(objects[0] ?? {}).length - 1}>
							<div class="flex flex-row gap-1 items-center">
								{key}
								<button
									class="p-1 w-6 h-6 flex justify-center items-center"
									on:click={() => {
										hiddenColumns = [...hiddenColumns, key]
									}}
									disabled={hiddenColumns.includes(key) ||
										hiddenColumns.length == Object.keys(objects[0] ?? {}).length - 1}
								>
									<EyeIcon size="16" class="hover:text-gray-600 text-gray-400 rounded-full " />
								</button>
								{#if activeSorting?.column === key}
									<button
										class="p-1 w-6 h-6 flex justify-center items-center"
										on:click={() => {
											activeSorting = {
												column: key,
												direction: activeSorting?.direction == 'asc' ? 'desc' : 'asc'
											}
										}}
										disabled={hiddenColumns.includes(key)}
									>
										{#if activeSorting?.direction == 'asc'}
											<ArrowDown size="16" />
										{:else}
											<ArrowUp size="16" />
										{/if}
									</button>
								{:else}
									<button
										class="p-1 w-6 h-6 flex justify-center items-center"
										on:click={() => {
											activeSorting = {
												column: key,
												direction: activeSorting?.direction == 'asc' ? 'desc' : 'asc'
											}
										}}
										disabled={hiddenColumns.includes(key)}
									>
										<ArrowDown size="16" class=" hover:text-gray-600 text-gray-400" />
									</button>
								{/if}
							</div>
						</Cell>
					{/each}
				</tr>
			</Head>
			<tbody class="divide-y">
				{#each data as row, index (index)}
					<Row>
						<Cell first={true} last={false} class="w-6">
							<input
								type="checkbox"
								class="!w-4 !h-4"
								value={selection.includes(row.id)}
								on:change={() => handleCheckboxChange(JSON.stringify(row))}
							/>
						</Cell>
						{#each Object.values(row ?? {}) ?? [] as value, index}
							<Cell first={index == 0} last={index == Object.values(row ?? {}).length - 1}>
								{#if Array.isArray(value)}
									<div class="flex flex-row gap-1">
										{#each value as item, index}
											<Badge color={badgeColors[index % badgeColors.length]}>
												{item}
											</Badge>
										{/each}
									</div>
								{:else}
									{value}
								{/if}
							</Cell>
						{/each}
					</Row>
				{/each}
			</tbody>
		</DataTable>
	{/key}
</div>
