<script lang="ts">
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext, BaseAppComponent, ButtonComponent } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import type { AppInput } from '../../inputType'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { writable } from 'svelte/store'
	import { createSvelteTable, flexRender, type TableOptions } from '@tanstack/svelte-table'
	import AppButton from '../buttons/AppButton.svelte'
	import { classNames, isObject } from '$lib/utils'
	import DebouncedInput from '../helpers/DebouncedInput.svelte'
	import AppTableFooter from './AppTableFooter.svelte'
	import RefreshButton from '../helpers/RefreshButton.svelte'
	import { tableOptions } from './tableOptions'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]

	export const staticOutputs: string[] = ['selectedRow', 'loading', 'result']

	type T = Record<string, any>

	$: result = [] as Array<Record<string, any>>

	let search: 'Runnable' | 'Component' | 'Disabled' = 'Disabled'
	let searchValue = ''

	let pagination: boolean | undefined = undefined
	let page = 1

	const options = writable<TableOptions<T>>({
		data: [],
		columns: [],
		...tableOptions
	})

	let table = createSvelteTable(options)

	const { worldStore, staticOutputs: staticOutputsStore } =
		getContext<AppEditorContext>('AppEditorContext')

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

	function setOptions(filteredResult: Array<Record<string, any>>) {
		if (!Array.isArray(result)) {
			return
		}

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

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value.toString().includes(searchValue))
		)
	}

	let filteredResult: Array<Record<string, any>> = []

	$: filteredResult && setOptions(filteredResult)
	$: extraQueryParams = search === 'Runnable' ? { search: searchValue, page } : { page, search: '' }
	$: search === 'Runnable' && (filteredResult = searchInResult(result, searchValue))
	$: (search === 'Runnable' || search === 'Disabled') && (filteredResult = result)
	$: outputs = $worldStore?.outputsById[id] as {
		selectedRow: Output<any>
	}

	function rerender() {
		table = createSvelteTable(options)
	}

	$: result && rerender()
</script>

<InputValue {id} input={configuration.search} bind:value={search} />
<InputValue {id} input={configuration.pagination} bind:value={pagination} />

<RunnableWrapper bind:componentInput {id} bind:result {extraQueryParams}>
	{#if Array.isArray(result) && result.every(isObject)}
		<div class="border border-gray-300 shadow-sm divide-y divide-gray-300  flex flex-col h-full">
			<div class="py-2 px-4">
				<div class="flex justify-between items-center">
					<RefreshButton componentId={id} />
					{#if search !== 'Disabled'}
						<div>
							<DebouncedInput placeholder="Search..." bind:value={searchValue} />
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
						{#each $table.getRowModel().rows as row, rowIndex (row.id)}
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
							>
								{#each row.getVisibleCells() as cell, index (index)}
									<td
										on:click={() => toggleRow(row, rowIndex)}
										class="p-4 whitespace-nowrap text-xs text-gray-900"
									>
										<svelte:component
											this={flexRender(cell.column.columnDef.cell, cell.getContext())}
										/>
									</td>
								{/each}

								{#if actionButtons.length > 0}
									<td
										class="flex w-full flex-row gap-2 p-4"
										on:click={() => toggleRow(row, rowIndex)}
									>
										{#each actionButtons as actionButton, actionIndex (actionIndex)}
											<AppButton
												noWFull
												{...actionButton}
												extraQueryParams={{ row: row.original }}
												bind:componentInput={actionButton.componentInput}
												bind:staticOutputs={$staticOutputsStore[actionButton.id]}
											/>
										{/each}
									</td>
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<AppTableFooter paginationEnabled={pagination} {result} {table} />
		</div>
	{:else}
		<Alert title="Parsing issues" type="error" size="xs">
			The result should be an array of objects
		</Alert>
	{/if}
</RunnableWrapper>
