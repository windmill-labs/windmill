<script lang="ts">
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext, BaseAppComponent, ButtonComponent } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import type { AppInput } from '../../inputType'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { writable } from 'svelte/store'
	import {
		createSvelteTable,
		flexRender,
		getCoreRowModel,
		type TableOptions,
		getPaginationRowModel
	} from '@tanstack/svelte-table'
	import AppButton from '../buttons/AppButton.svelte'
	import { classNames } from '$lib/utils'
	import Button from '$lib/components/common/button/Button.svelte'
	import { faDownload, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import DebouncedInput from '../helpers/DebouncedInput.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]

	type T = Record<string, any>

	$: result = [] as Array<Record<string, any>>

	let search: 'Frontend' | 'Backend' | 'Disabled' = 'Disabled'
	let pagination: boolean | undefined = undefined

	let page = 1
	let searchValue = ''

	const tableOptions = {
		getCoreRowModel: getCoreRowModel(),
		getPaginationRowModel: getPaginationRowModel(),
		initialState: {
			pagination: {
				pageSize: 10
			}
		}
	}

	const options = writable<TableOptions<T>>({
		data: result,
		columns: [],
		...tableOptions
	})

	function setOptions(filteredResult: Array<Record<string, any>>) {
		const headers = Array.from(new Set(result.flatMap((row) => Object.keys(row))))

		$options = {
			data: filteredResult,
			columns: headers.map((header) => {
				return {
					accessorKey: header,
					cell: (info) => info.getValue()
				}
			}),
			...tableOptions
		}
	}

	$: filteredResult && setOptions(filteredResult)

	const table = createSvelteTable(options)

	const {
		worldStore,
		staticOutputs: staticOutputsStore,
		runnableComponents
	} = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = ['selectedRow', 'loading', 'result']

	$: outputs = $worldStore?.outputsById[id] as {
		selectedRow: Output<any>
	}

	let selectedRowIndex = -1

	function toggleRow(row: Record<string, any>, rowIndex: number) {
		if (selectedRowIndex === rowIndex) {
			selectedRowIndex = -1
			outputs.selectedRow.set(null)
		} else {
			selectedRowIndex = rowIndex
			outputs?.selectedRow.set(row.original)
		}
	}

	$: extraQueryParams = search === 'Backend' ? { search: searchValue, page } : { page, search: '' }

	$: console.log({ extraQueryParams })

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value.toString().includes(searchValue))
		)
	}

	let filteredResult: Array<Record<string, any>> = []

	$: search === 'Frontend' && (filteredResult = searchInResult(result, searchValue))
	$: (search === 'Backend' || search === 'Disabled') && (filteredResult = result)

	let loading = false
	async function refresh() {
		loading = true
		await $runnableComponents[id]?.()
		loading = false
	}
</script>

<InputValue input={configuration.search} bind:value={search} />
<InputValue input={configuration.pagination} bind:value={pagination} />

<RunnableWrapper bind:componentInput {id} bind:result {extraQueryParams}>
	<div class="border border-gray-300 shadow-sm divide-y divide-gray-300  flex flex-col h-full">
		<div class="py-2 px-4">
			<div class="flex justify-between items-center">
				<Button
					iconOnly
					startIcon={{ icon: faRefresh, classes: loading ? 'animate-spin' : '' }}
					color="dark"
					size="xs"
					on:click={refresh}
				/>
				{#if search !== 'Disabled'}
					<div>
						<div>
							<DebouncedInput placeholder="Search..." bind:value={searchValue} />
						</div>
					</div>
				{/if}
			</div>
		</div>
		<div class="overflow-auto flex-1 w-full">
			<table class="divide-y divide-gray-300 w-full border-b border-b-gray-200">
				<thead class="bg-gray-50 text-left">
					{#each $table.getHeaderGroups() as headerGroup}
						<tr class="divide-x">
							{#each headerGroup.headers as header}
								<th class="px-4 py-4 text-sm font-semibold">
									{#if !header.isPlaceholder}
										<svelte:component
											this={flexRender(header.column.columnDef.header, header.getContext())}
										/>
									{/if}
								</th>
							{/each}
							{#if actionButtons.length > 0}
								<th class="px-4 py-4 text-sm font-semibold">Actions</th>
							{/if}
						</tr>
					{/each}
				</thead>
				<tbody class="divide-y divide-gray-200 bg-white ">
					{#each $table.getRowModel().rows as row, rowIndex}
						<tr
							class={classNames(
								selectedRowIndex === rowIndex
									? 'bg-blue-100 hover:bg-blue-200'
									: 'hover:bg-blue-50',
								'divide-x',
								'border-b w-full',
								selectedRowIndex === rowIndex
									? 'divide-blue-200 hover:divide-blue-300'
									: 'divide-gray-200'
							)}
							on:click={() => toggleRow(row, rowIndex)}
						>
							{#each row.getVisibleCells() as cell}
								<td class="p-4 whitespace-nowrap text-xs text-gray-900">
									<svelte:component
										this={flexRender(cell.column.columnDef.cell, cell.getContext())}
									/>
								</td>
							{/each}

							{#if actionButtons.length > 0}
								<td>
									{#each actionButtons as props}
										<AppButton
											{...props}
											extraQueryParams={{ row }}
											bind:componentInput={props.componentInput}
											bind:staticOutputs={$staticOutputsStore[props.id]}
										/>
									{/each}
								</td>
							{/if}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<div class="px-4 py-2 text-xs flex flex-row gap-2 items-center justify-between">
			{#if pagination}
				<div class="flex items-center gap-2 flex-row">
					<Button
						size="xs"
						variant="border"
						color="light"
						on:click={() => $table.previousPage()}
						disabled={!$table.getCanPreviousPage()}
					>
						Previous
					</Button>
					<Button
						size="xs"
						variant="border"
						color="light"
						on:click={() => $table.nextPage()}
						disabled={!$table.getCanNextPage()}
					>
						Next
					</Button>
					{$table.getState().pagination.pageIndex + 1} of {$table.getPageCount()}
				</div>
			{:else}
				<div />
			{/if}
			<div class="flex items-center gap-2 flex-row">
				<Button
					size="xs"
					variant="border"
					color="light"
					on:click={() => {
						const dataStr =
							'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(result))
						const downloadAnchorNode = document.createElement('a')
						downloadAnchorNode.setAttribute('href', dataStr)
						downloadAnchorNode.setAttribute('download', 'data.json')
						document.body.appendChild(downloadAnchorNode) // required for firefox
						downloadAnchorNode.click()
						downloadAnchorNode.remove()
					}}
					startIcon={{ icon: faDownload }}
				>
					Download
				</Button>
			</div>
		</div>
	</div>
</RunnableWrapper>
