<script lang="ts">
	import { run } from 'svelte/legacy'

	import { ArrowDown, ArrowUp, Download, MoreVertical, MoveVertical, Columns } from 'lucide-svelte'
	import Dropdown from '../DropdownV2.svelte'
	import Cell from './Cell.svelte'
	import DataTable from './DataTable.svelte'
	import Head from './Head.svelte'
	import Row from './Row.svelte'
	import { pluralize } from '$lib/utils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import {
		computeStructuredObjectsAndHeaders,
		convertJsonToCsv,
		isEmail,
		isLink
	} from './tableUtils'
	import type { BadgeColor } from '../common'
	import Popover from '../Popover.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import DownloadCsv from './DownloadCsv.svelte'
	interface Props {
		objects?: Array<Record<string, any>>
		class?: string
	}

	let { objects = [], class: className }: Props = $props()

	let currentPage = $state(1)
	let perPage = $state(25)
	let search: string = $state('')

	let structuredObjects: {
		_id: number
		rowData: Record<string, any>
	}[] = $state([])
	let headers: string[] = $state([])

	function recomputeObjectsAndHeaders(objects: Array<Record<string, any>>) {
		;[headers, structuredObjects] = computeStructuredObjectsAndHeaders(objects)
	}

	function adjustCurrentPage() {
		const totalItems = objects.length
		const totalPages = Math.ceil(totalItems / perPage)
		if (currentPage > totalPages) {
			currentPage = totalPages > 0 ? totalPages : 1
		}
	}

	type ActiveSorting = {
		column: string
		direction: 'asc' | 'desc'
	}

	let activeSorting: ActiveSorting | undefined = $state(undefined)

	function sortObjects(
		activeSorting: ActiveSorting | undefined,
		objects: Array<Record<string, any>>,
		getRowData: boolean
	) {
		if (activeSorting) {
			return objects.sort((a, b) => {
				if (!activeSorting) return 0
				const valA = (getRowData ? a.rowData : a)?.[activeSorting.column]
				const valB = (getRowData ? b.rowData : b)?.[activeSorting.column]
				const isAsc = activeSorting.direction === 'asc'
				if (valA == undefined || valA == null) {
					return isAsc ? -1 : 1
				}
				if (valB == undefined || valB == null) {
					return isAsc ? 1 : -1
				}
				if (isAsc) {
					return valA > valB ? 1 : -1
				} else {
					return valA > valB ? -1 : 1
				}
			})
		} else {
			return objects
		}
	}
	function computeData(
		structuredObjects: Array<Record<string, any>>,
		activeSorting: ActiveSorting | undefined,
		search: string
	): Array<Record<string, any>> {
		let objects = structuredObjects
		if (search != undefined && search != '') {
			objects = objects.filter((obj) =>
				Object.values(obj.rowData).some((value) =>
					JSON.stringify(value).toLowerCase().includes(search.toLowerCase())
				)
			)
		} else {
			objects = [...objects]
		}
		return sortObjects(activeSorting, objects, true)
	}

	let selection = $state([] as Array<number>)
	let colSelection = $state([] as Array<string>)

	// Function to handle individual row checkbox change
	function handleCheckboxChange(rowId: number) {
		if (selection.includes(rowId)) {
			// Remove the id from the selection array
			selection = selection.filter((id) => id !== rowId)
		} else {
			// Add the id to the selection array
			selection = [...selection, rowId]
		}
	}

	// Function to handle select all checkbox change
	function handleSelectAllChange() {
		if (
			selection.length === 0 ||
			(selection.length < slicedData.length &&
				(colSelection.length === 0 || colSelection.length < headers.length))
		) {
			// Select all rows
			selection = slicedData.map((row) => row._id)
			colSelection = [...headers]
		} else {
			// Deselect all rows
			selection = []
			colSelection = []
		}
	}

	let renderCount = $state(0)

	const badgeColors: BadgeColor[] = ['gray', 'blue', 'green', 'yellow', 'indigo']
	const darkBadgeColors: BadgeColor[] = [
		'dark-gray',
		'dark-blue',
		'dark-green',
		'dark-yellow',
		'dark-indigo'
	]
	let darkMode = $state(false)
	let wrapperWidth = $state(0)

	// function isSortable(key: string) {
	// 	let value = objects?.[0]?.[key]
	// 	let typof = typeof value
	// 	return (value != undefined && typof === 'string') || typof === 'number' || typof === 'boolean'
	// }

	function handleColumnSelected(key: string) {
		if (colSelection.includes(key)) {
			// Remove the id from the colSelection array
			colSelection = colSelection.filter((id) => id !== key)
		} else {
			// Add the id to the colSelection array
			colSelection = [...colSelection, key]
		}
	}
	run(() => {
		recomputeObjectsAndHeaders(objects)
	})
	run(() => {
		perPage && adjustCurrentPage()
	})
	let data = $derived(computeData(structuredObjects, activeSorting, search))
	let slicedData = $derived(data.slice((currentPage - 1) * perPage, currentPage * perPage))
</script>

<DarkModeObserver bind:darkMode />

<div class={className} bind:clientWidth={wrapperWidth}>
	<div class="flex flex-col gap-2 py-1 my-1" style={`max-width: ${wrapperWidth}px;`}>
		<div class="flex flex-row justify-between items-center gap-2">
			<div class="flex flex-row gap-2 items-center whitespace-nowrap w-full">
				<input bind:value={search} placeholder="Search..." class="h-8 !text-xs" />
				{#if selection.length > 0}
					<span class="text-xs text-gray-500 dark:text-gray-200">
						{pluralize(selection?.length ?? 1, 'item') + ' selected'}
					</span>
				{/if}
			</div>
			<div class="flex flex-row items-center gap-2">
				<DownloadCsv
					getContent={() => {
						return convertJsonToCsv(
							selection.length > 0
								? sortObjects(activeSorting, structuredObjects, true)
										.filter(({ _id }) => selection.includes(_id))
										.map((obj) =>
											colSelection.length == 0
												? obj.rowData
												: Object.fromEntries(
														Object.entries(obj.rowData).filter(([key, _]) =>
															colSelection.includes(key)
														)
													)
										)
								: colSelection.length == 0
									? sortObjects(activeSorting, objects, false)
									: sortObjects(activeSorting, objects, false).map((obj) =>
											Object.fromEntries(
												Object.entries(obj).filter(([key, _]) => colSelection.includes(key))
											)
										)
						)
					}}
					customText={selection.length > 0 || colSelection.length > 0
						? 'Download selected as CSV'
						: undefined}
				/>
				<Dropdown
					items={() => {
						const actions = [
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
							}
						]

						if (selection.length > 0) {
							actions.push({
								displayName: 'Clear selection',
								icon: Columns,
								action: () => {
									colSelection = []
									selection = []
									renderCount++
								}
							})
						}

						return actions
					}}
				>
					{#snippet buttonReplacement()}
						<MoreVertical
							size={8}
							class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
						/>
					{/snippet}
				</Dropdown>
			</div>
		</div>
		{#key renderCount}
			{#if data.length == 0}
				<div class="flex flex-col items-center justify-center border rounded-md py-8">
					<div class="text-gray-500 dark:text-gray-200 text-sm"> No data found </div>
					<div class="text-gray-500 dark:text-gray-200 text-xs">
						Try changing your search query
					</div>
				</div>
			{:else}
				<DataTable
					size="sm"
					shouldHidePagination={false}
					paginated={true}
					bind:currentPage
					bind:perPage
					on:next={() => {
						currentPage += 1
					}}
					on:previous={() => {
						currentPage -= 1
					}}
					on:change={(event) => {
						currentPage = event.detail
					}}
					showNext={objects.length > perPage}
					hasMore={currentPage * perPage < objects.length}
					rowCount={data.length}
				>
					<Head>
						<tr>
							<Cell head first={true} last={false}>
								<input type="checkbox" class="!w-4 !h-4" onchange={handleSelectAllChange} />
							</Cell>
							{#each headers ?? [] as key, index}
								<Cell head last={index == headers.length - 1}>
									<div class="flex flex-row gap-1 items-center">
										{key}
										{#if activeSorting?.column === key}
											<button
												class="p-1 w-6 h-6 flex justify-center items-center"
												onclick={() => {
													activeSorting = {
														column: key,
														direction: activeSorting?.direction == 'asc' ? 'desc' : 'asc'
													}
												}}
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
												onclick={() => {
													activeSorting = {
														column: key,
														direction: activeSorting?.direction == 'asc' ? 'desc' : 'asc'
													}
												}}
											>
												<MoveVertical size="16" class=" hover:text-gray-600 text-gray-400" />
											</button>
										{/if}
										<input
											type="checkbox"
											class="!w-4 !h-4"
											checked={colSelection.includes(key)}
											onchange={() => handleColumnSelected(key)}
										/>
									</div>
								</Cell>
							{/each}
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each slicedData.filter((x) => x) as { _id, rowData }, index (index)}
							<Row dividable selected={selection.includes(_id) && colSelection.length == 0}>
								<Cell first={true} last={false} class="w-6">
									<input
										type="checkbox"
										class="!w-4 !h-4"
										checked={selection.includes(_id)}
										onchange={() => handleCheckboxChange(_id)}
									/>
								</Cell>
								{#each headers as key, index}
									{@const value = rowData[key]}
									<Cell
										selected={colSelection.includes(key) &&
											(selection.length == 0 || selection.includes(_id))}
										last={index == Object.values(rowData ?? {}).length - 1}
									>
										{#if Array.isArray(value) && value.length === 0}
											<div></div>
										{:else if Array.isArray(value) && typeof value?.[0] === 'string'}
											<div class="flex flex-row gap-1 w-full max-w-32 flex-wrap min-w-32">
												{#each value as item, index}
													<Badge
														color={darkMode
															? darkBadgeColors[index % darkBadgeColors.length]
															: badgeColors[index % badgeColors.length]}
													>
														{item}
													</Badge>
												{/each}
											</div>
										{:else if Array.isArray(value) && typeof value?.[0] === 'number'}
											<div class="flex flex-row gap-1 w-full max-w-32 flex-wrap min-w-32">
												{#each value as val}
													<div
														class="p-2 bg-surface-secondary rounded-md text-2xs text-wrap whitespace-pre-wrap flex flex-grow w-max overflow-hidden"
													>
														{JSON.stringify(val, null, 2)}
													</div>
												{/each}
											</div>
										{:else if Array.isArray(value)}
											<div class="flex flex-row gap-1 w-full flex-wrap min-w-96">
												{#each value as val}
													<div
														class="p-2 bg-surface-secondary rounded-md text-2xs text-wrap whitespace-pre-wrap flex flex-grow w-max overflow-hidden"
													>
														{JSON.stringify(val, null, 2)}
													</div>
												{/each}
											</div>
										{:else if typeof value === 'string' && isEmail(value)}
											<a href={`mailto:${value}`} class="hover:underline">
												{value}
											</a>
										{:else if typeof value === 'string' && isLink(value)}
											<a href={value} target="_blank" class="hover:underline">
												{value}
											</a>
										{:else}
											{@const txt =
												value == undefined || value == null
													? ''
													: typeof value != 'string'
														? JSON.stringify(value)
														: value}
											<Popover
												placement="bottom"
												notClickable
												disablePopup={typeof value === 'string' && value.length < 100}
											>
												<div
													class="max-w-[100ch] text-wrap whitespace-pre-wrap flex flex-grow w-max three-lines cursor-text"
												>
													{txt?.length > 100 ? txt.slice(0, 100) + '...' : txt}
												</div>
												{#snippet text()}
													{txt}
												{/snippet}
											</Popover>
										{/if}
									</Cell>
								{/each}
							</Row>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		{/key}
	</div>
</div>

<style>
	.three-lines {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
