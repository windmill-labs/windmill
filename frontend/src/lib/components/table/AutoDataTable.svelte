<script lang="ts">
	import {
		ArrowDown,
		ArrowUp,
		Download,
		EyeIcon,
		MoreVertical,
		MoveVertical,
		Columns,
		EyeOff
	} from 'lucide-svelte'
	import Dropdown from '../DropdownV2.svelte'
	import Cell from './Cell.svelte'
	import DataTable from './DataTable.svelte'
	import Head from './Head.svelte'
	import Row from './Row.svelte'
	import { pluralize } from '$lib/utils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { isEmail, isLink } from './tableUtils'
	import type { BadgeColor } from '../common'
	import Popover from '../Popover.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import Button from '../common/button/Button.svelte'

	export let objects: Array<Record<string, any>> = []

	let currentPage = 1
	let perPage = 5
	let search: string = ''

	let nextId = 1
	$: structuredObjects = objects.map((obj) => {
		return {
			_id: nextId++,
			rowData: { ...obj }
		}
	})

	$: data = structuredObjects
		.filter(({ rowData }) =>
			Object.values(rowData).some((value) =>
				JSON.stringify(value).toLowerCase().includes(search.toLowerCase())
			)
		)
		.sort((a, b) => {
			if (!activeSorting) return 0
			const valA = a.rowData[activeSorting.column]
			const valB = b.rowData[activeSorting.column]
			if (activeSorting.direction === 'asc') {
				return valA > valB ? 1 : -1
			} else {
				return valA < valB ? 1 : -1
			}
		})
		.slice((currentPage - 1) * perPage, currentPage * perPage)

	let hiddenColumns = [] as Array<string>
	let activeSorting:
		| {
				column: string
				direction: 'asc' | 'desc'
		  }
		| undefined = undefined

	let selection = [] as Array<number>

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
		if (selection.length === 0 || selection.length < data.length) {
			// Select all rows
			selection = data.map((row) => row._id)
		} else {
			// Deselect all rows
			selection = []
		}
		selection = [...selection]
	}

	let renderCount = 0

	const badgeColors: BadgeColor[] = ['gray', 'blue', 'red', 'green', 'yellow', 'indigo']
	const darkBadgeColors: BadgeColor[] = [
		'dark-gray',
		'dark-blue',
		'dark-red',
		'dark-green',
		'dark-yellow',
		'dark-indigo'
	]
	let darkMode = false
	let wrapperWidth = 0

	function isSortable(key: string) {
		return (
			typeof objects[0][key] === 'string' ||
			typeof objects[0][key] === 'number' ||
			typeof objects[0][key] === 'boolean'
		)
	}
</script>

<DarkModeObserver bind:darkMode />

<div class="w-full" bind:clientWidth={wrapperWidth}>
	<div class="flex flex-col gap-2 py-4 my-4" style={`max-width: ${wrapperWidth}px;`}>
		<div class="flex flex-row justify-between items-center">
			<div class="flex flex-row gap-2 items-center whitespace-nowrap w-full">
				<input bind:value={search} placeholder="Search..." class="h-8 !text-xs !w-80" />
				{#if selection.length > 0}
					<span class="text-xs text-gray-500 dark:text-gray-200">
						{pluralize(selection?.length ?? 1, 'item') + ' selected'}
					</span>
				{/if}
				{#if hiddenColumns.length > 0}
					<div class="flex flex-row gap-2 justify-center items-center mx-2">
						<span class="text-xs text-gray-500 dark:text-gray-200" />
						<Button
							size="xs2"
							color="light"
							variant="border"
							on:click={() => {
								hiddenColumns = []
							}}
							startIcon={{
								icon: Columns
							}}
						>
							Display hidden columns ({pluralize(hiddenColumns?.length ?? 1, 'column')})
						</Button>
					</div>
				{/if}
			</div>
			<div class="flex flex-row items-center gap-2">
				<Button
					size="xs"
					color="light"
					startIcon={{ icon: Download }}
					on:click={() => {
						const headers =
							structuredObjects.length > 0
								? Object.keys(structuredObjects[0].rowData).join(',')
								: ''
						const csvContent = [
							headers, // Add headers as the first row
							...structuredObjects
								.filter(({ _id }) => {
									if (selection.length > 0) {
										return selection.includes(_id)
									} else {
										return true
									}
								})
								.map(({ rowData }) => Object.values(rowData).join(','))
						].join('\n')

						const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
						const url = URL.createObjectURL(blob)
						const link = document.createElement('a')
						link.setAttribute('href', url)
						link.setAttribute('download', 'data.csv')
						link.style.visibility = 'hidden'
						document.body.appendChild(link)
						link.click()

						document.body.removeChild(link)
					}}
				>
					{#if selection.length > 0}
						Download selected as CSV
					{:else}
						Download as CSV
					{/if}
				</Button>
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

						if (hiddenColumns.length > 0) {
							actions.push({
								displayName: 'Display hidden columns',
								icon: EyeIcon,
								action: () => {
									hiddenColumns = []
								}
							})
						}

						if (selection.length > 0) {
							actions.push({
								displayName: 'Clear selection',
								icon: Columns,
								action: () => {
									selection = []
									renderCount++
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
					showNext={currentPage * perPage < objects.length}
				>
					<Head>
						<tr>
							<Cell head first={true} last={false}>
								<input type="checkbox" class="!w-4 !h-4" on:change={handleSelectAllChange} />
							</Cell>
							{#each Object.keys(data[0].rowData ?? {}) ?? [] as key, index}
								<Cell head last={index == Object.keys(objects[0] ?? {}).length - 1}>
									<div class="flex flex-row gap-1 items-center">
										{key}
										{#if hiddenColumns.includes(key)}
											<button
												class="p-1 w-6 h-6 flex justify-center items-center"
												on:click={() => {
													hiddenColumns = hiddenColumns.filter((col) => col !== key)
												}}
											>
												<EyeOff size="16" class="hover:text-gray-600 text-gray-400 rounded-full " />
											</button>
										{:else}
											<button
												class="p-1 w-6 h-6 flex justify-center items-center"
												on:click={() => {
													hiddenColumns = [...hiddenColumns, key]
												}}
											>
												<EyeIcon
													size="16"
													class="hover:text-gray-600 text-gray-400 rounded-full "
												/>
											</button>
										{/if}
										{#if isSortable(key)}
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
													<MoveVertical size="16" class=" hover:text-gray-600 text-gray-400" />
												</button>
											{/if}
										{/if}
									</div>
								</Cell>
							{/each}
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each data as { _id, rowData }, index (index)}
							<Row dividable selected={selection.includes(_id)}>
								<Cell first={true} last={false} class="w-6">
									<input
										type="checkbox"
										class="!w-4 !h-4"
										checked={selection.includes(_id)}
										on:change={() => handleCheckboxChange(_id)}
									/>
								</Cell>
								{#each Object.keys(rowData ?? {}) ?? [] as key, index}
									{@const value = rowData[key]}
									<Cell last={index == Object.values(rowData ?? {}).length - 1}>
										{#if hiddenColumns.includes(key)}
											...
										{:else if Array.isArray(value) && typeof value[0] === 'string'}
											<div class="flex flex-row gap-1 w-full max-w-80 flex-wrap min-w-80">
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
										{:else if Array.isArray(value)}
											<div class="flex flex-row gap-1 w-full max-w-80 flex-wrap min-w-96">
												{#each value as val}
													<div
														class="p-2 bg-surface-secondary rounded-md text-2xs max-w-96 text-wrap whitespace-pre-wrap flex flex-grow w-max overflow-hidden"
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
											{@const txt = typeof value == 'object' ? JSON.stringify(value) : value}
											<Popover
												placement="bottom"
												notClickable
												disablePopup={typeof value === 'string' && value.length < 100}
											>
												<div
													class="max-w-80 text-wrap whitespace-pre-wrap flex flex-grow w-max three-lines cursor-text"
												>
													{txt.length > 100 ? txt.slice(0, 100) + '...' : txt}
												</div>
												<svelte:fragment slot="text">{txt}</svelte:fragment>
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
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
