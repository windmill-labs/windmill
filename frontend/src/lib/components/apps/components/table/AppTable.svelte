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
	import { tableOptions } from './tableOptions'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]

	export const staticOutputs: string[] = ['selectedRow', 'loading', 'result', 'search']

	type T = Record<string, any>

	$: result = [] as Array<Record<string, any>>

	let search: 'By Runnable' | 'By Component' | 'Disabled' | undefined = undefined
	let searchValue = ''

	let pagination: boolean | undefined = true

	$: setSearch(searchValue)

	function setSearch(srch) {
		outputs?.search?.set(srch)
	}

	const options = writable<TableOptions<T>>({
		...tableOptions,
		data: [],
		columns: []
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
			...tableOptions,
			data: filteredResult,
			columns: headers.map((header) => {
				return {
					accessorKey: header,
					cell: (info) => info.getValue()
				}
			})
		}
	}

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value?.toString()?.includes(searchValue))
		)
	}

	function renderCell(x: any, props: any) {
		try {
			return flexRender(x, props)
		} catch (e) {
			return undefined
		}
	}

	function cellIsObject(x: (any) => any, props: any): boolean {
		return typeof x != 'string' && typeof x(props) == 'object'
	}

	let filteredResult: Array<Record<string, any>> = []

	$: filteredResult && setOptions(filteredResult)
	$: search === 'By Component' && (filteredResult = searchInResult(result, searchValue))
	$: (search === 'By Runnable' || search === 'Disabled') && (filteredResult = result)
	$: outputs = $worldStore?.outputsById[id] as {
		selectedRow: Output<any>
		search: Output<string>
	}

	function rerender() {
		table = createSvelteTable(options)
	}

	$: result && rerender()
</script>

<InputValue {id} input={configuration.search} bind:value={search} />

<RunnableWrapper bind:componentInput {id} bind:result>
	{#if Array.isArray(result) && result.every(isObject)}
		<div class="border border-gray-300 shadow-sm divide-y divide-gray-300  flex flex-col h-full">
			{#if search !== 'Disabled'}
				<div class="px-4 py-2">
					<div class="flex items-center">
						<div>
							<DebouncedInput placeholder="Search..." bind:value={searchValue} />
						</div>
					</div>
				</div>
			{/if}

			<div class="overflow-auto flex-1 w-full">
				<table class="divide-y divide-gray-300 w-full border-b border-b-gray-200">
					<thead class="bg-gray-50 text-left">
						{#each $table.getHeaderGroups() as headerGroup}
							<tr class="divide-x">
								{#each headerGroup.headers as header}
									{#if header?.column?.columnDef?.header}
										{@const context = header?.getContext()}
										{#if context}
											{@const component = renderCell(header.column.columnDef.header, context)}
											<th class="px-4 py-4 text-sm font-semibold">
												{#if !header.isPlaceholder && component}
													<svelte:component this={component} />
												{/if}
											</th>
										{/if}
									{/if}
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
									{#if cell?.column?.columnDef?.cell}
										{@const context = cell?.getContext()}
										{#if context}
											{@const component = renderCell(cell.column.columnDef.cell, context)}
											<td
												on:click={() => toggleRow(row, rowIndex)}
												class="p-4 whitespace-pre-wrap truncate text-xs text-gray-900"
											>
												{#if typeof cell.column.columnDef.cell != 'string' && cellIsObject(cell.column.columnDef.cell, context)}
													{JSON.stringify(cell.column.columnDef.cell(context), null, 4)}
												{:else if component != undefined}
													<svelte:component this={component} />
												{/if}
											</td>
										{/if}
									{/if}
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
