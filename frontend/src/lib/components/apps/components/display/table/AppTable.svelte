<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, onDestroy, onMount, untrack } from 'svelte'
	import type {
		AppViewerContext,
		BaseAppComponent,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../../types'
	import type { AppInput } from '../../../inputType'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { writable } from 'svelte/store'

	// tanstack-table v8 does not support svelte 5.
	//
	// This packages acts as a drop-in replacement:
	// https://github.com/dummdidumm/tanstack-table-8-svelte-5
	// It re-exports the core lib types but for some reason I can't
	// import them so I get them manually from node_modules.
	//
	// tanstack-table v9 supports svelte 5 but is still in alpha at the
	// time of writing, and introduces unstable breaking changes
	import { createSvelteTable, flexRender } from '@tanstack/svelte-table'
	import type {
		HeaderGroup,
		Row,
		Table,
		TableOptions
	} from '$lib/../../node_modules/@tanstack/table-core/build/lib'

	import AppButton from '../../buttons/AppButton.svelte'
	import { classNames, isObject, sendUserToast } from '$lib/utils'
	import DebouncedInput from '../../helpers/DebouncedInput.svelte'
	import AppTableFooter from './AppTableFooter.svelte'
	import { tableOptions } from './tableOptions'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import {
		components,
		type ButtonComponent,
		type CheckboxComponent,
		type SelectComponent
	} from '../../../editor/component'
	import { initCss } from '../../../utils'
	import { twMerge } from 'tailwind-merge'
	import { connectOutput, initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import AppCheckbox from '../../inputs/AppCheckbox.svelte'
	import AppSelect from '../../inputs/AppSelect.svelte'
	import RowWrapper from '../../layout/RowWrapper.svelte'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import ComponentOutputViewer from '$lib/components/apps/editor/contextPanel/ComponentOutputViewer.svelte'
	import { EyeIcon, Plug2 } from 'lucide-svelte'
	import AppCell from './AppCell.svelte'
	import sum from 'hash-sum'
	import RefreshButton from '$lib/components/apps/components/helpers/RefreshButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		actionButtons: (BaseAppComponent & (ButtonComponent | CheckboxComponent | SelectComponent))[]
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'tablecomponent'> | undefined
		render: boolean
	}

	let {
		id,
		componentInput,
		configuration,
		actionButtons,
		initializing = $bindable(undefined),
		customCss = undefined,
		render
	}: Props = $props()

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	type T = Record<string, any>

	let result: Record<string, any>[] | undefined = $state(undefined)

	const {
		app,
		worldStore,
		componentControl,
		selectedComponent,
		hoverStore,
		mode,
		connectingInput
	} = getContext<AppViewerContext>('AppViewerContext')

	let searchValue = $state('')

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: undefined,
		loading: false,
		result: [] as Record<string, any>[],
		inputs: {},
		search: '',
		page: 1
	})

	let inputs = $state({})
	let loading: boolean = $state(false)

	function setSearch(srch: string) {
		$table.setPageIndex(0)
		outputs?.search?.set(srch)
	}

	const options = writable<TableOptions<T>>({
		...tableOptions,
		data: [],
		columns: []
	})

	let table = $state(createSvelteTable(options))

	let resolvedConfig = $state(
		initConfig(components['tablecomponent'].initialData.configuration, configuration)
	)

	let selectedRowIndex = $state(-1)

	function toggleRow(row: Record<string, any>, force: boolean = false) {
		let data = { ...row.original }
		let index = data['__index']
		delete data['__index']
		let peak = outputs?.selectedRow.peak()

		if (selectedRowIndex !== index || peak == undefined || sum(data) != sum(peak) || force) {
			selectedRowIndex = index
			outputs?.selectedRow.set(data, force)
			outputs?.selectedRowIndex.set(index, force)
			if (iterContext && listInputs) {
				listInputs.set(id, { selectedRow: data, selectedRowIndex: index })
			}
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let mounted = $state(false)
	onMount(() => {
		mounted = true
	})

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) => {
			// console.log(Object.values(row).map((value) => value?.toString()))
			return Object.values(row).some((value) => value?.toString()?.includes(searchValue))
		})
	}

	function renderCell(x: any, props: any): any {
		try {
			return flexRender(x, props)
		} catch (e) {
			return undefined
		}
	}

	let filteredResult: Array<Record<string, any>> = $state([])

	function setFilteredResult() {
		console.log('setting filtered result')
		const wIndex = Array.isArray(result)
			? (result as any[]).map((x, i) => ({ ...x, __index: i }))
			: [{ error: 'input was not an array' }]

		if (resolvedConfig.search === 'By Runnable' || resolvedConfig.search === 'Disabled') {
			filteredResult = wIndex ?? []
		} else {
			filteredResult = searchInResult(wIndex ?? [], searchValue)
		}
	}

	function rerender() {
		if (!Array.isArray(result)) {
			return
		}

		const headers = Array.from(
			new Set(result.flatMap((row) => (typeof row == 'object' ? Object.keys(row ?? {}) : [])))
		)

		$options = {
			...tableOptions,
			...(resolvedConfig?.pagination?.selected != 'manual'
				? {
						initialState: {
							...resolvedConfig.initialState,
							pagination: {
								pageSize: resolvedConfig?.pagination?.configuration?.auto?.pageSize ?? 20
							}
						}
					}
				: {}),
			manualPagination: resolvedConfig?.pagination?.selected == 'manual',
			pageCount:
				resolvedConfig?.pagination?.selected == 'manual'
					? (resolvedConfig?.pagination?.configuration.manual.pageCount ?? -1)
					: undefined,
			data: filteredResult,
			columns: headers.map((header) => {
				return {
					accessorKey: header,
					cell: (info) => info.getValue()
				}
			})
		}
		table = createSvelteTable(options)

		if (result) {
			//console.log('rerendering table', result[0])
			if (resolvedConfig?.selectFirstRowByDefault != false) {
				toggleRow({ original: filteredResult[0] }, true)
			}
		}

		if (outputs.page.peak()) {
			$table.setPageIndex(outputs?.page.peak())
		}
	}

	let css = $state(initCss($app.css?.tablecomponent, customCss))

	$componentControl[id] = {
		right: (skipActions: boolean | undefined) => {
			if (skipActions) {
				return false
			}

			const hasActions = actionButtons.length >= 1

			if (hasActions) {
				$selectedComponent = [actionButtons[0].id]
				return true
			}
			return false
		},
		setSelectedIndex: (index: number) => {
			if (filteredResult) {
				toggleRow({ original: filteredResult[index] }, true)
			}
		},
		setValue(nvalue) {
			if (Array.isArray(nvalue)) {
				result = nvalue
				outputs?.result.set(nvalue)
			}
		}
	}

	function getHeaderGroups<T>(table: Table<T>): Array<HeaderGroup<T>> {
		try {
			return table.getHeaderGroups()
		} catch (e) {
			sendUserToast("Couldn't render table header groups: " + e, true)
			console.error(e)
			return []
		}
	}

	function getDisplayNameById(id: string) {
		const component = resolvedConfig?.columnDefs?.find((columnDef) => columnDef.field === id)
		return component?.headerName
	}

	function safeVisibleCell<T>(row: Row<T>) {
		try {
			return row.getVisibleCells()
		} catch (e) {
			sendUserToast("Couldn't render table header groups: " + e, true)
			console.error(e)
			return []
		}
	}

	function updateTable(resolvedConfig) {
		if (resolvedConfig?.columnDefs) {
			$table.getAllLeafColumns().map((column) => {
				const columnConfig = resolvedConfig.columnDefs.find(
					// @ts-ignore
					(columnDef) => columnDef.field === column.columnDef.accessorKey
				)
				if (columnConfig?.hideColumn === column.getIsVisible()) {
					column.toggleVisibility()
				}
			})
			const newColumnOrder = resolvedConfig.columnDefs.map(
				(columnDef: { field: any }) => columnDef.field
			)

			$table.setColumnOrder(() => newColumnOrder)
		}
	}

	function updateCellValue(rowIndex: number, columnIndex: number, newCellValue: string) {
		if (result && rowIndex < result.length) {
			const updatedRow = { ...result[rowIndex] }
			const columnName = $table?.getAllLeafColumns()?.[columnIndex]?.columnDef?.['accessorKey']

			if (!columnName) {
				sendUserToast(`Column name for index ${columnIndex} is not defined.`, true)
				return
			}

			updatedRow[columnName] = newCellValue
			result[rowIndex] = updatedRow
			outputs?.result.set([result])
		}
	}
	$effect(() => {
		;[searchValue]
		untrack(() => setSearch(searchValue))
	})
	$effect(() => {
		resolvedConfig?.selectFirstRowByDefault != false &&
			selectedRowIndex === -1 &&
			Array.isArray(result) &&
			result.length > 0 &&
			// We need to wait until the component is mounted so the world is created
			mounted &&
			outputs &&
			toggleRow({ original: { ...result[0], __index: 0 } })
	})
	$effect(() => {
		;[result, resolvedConfig.search, searchValue, resolvedConfig.pagination]
		untrack(() => setFilteredResult())
	})
	$effect(() => {
		outputs.page.set($table.getState().pagination.pageIndex)
	})
	$effect(() => {
		filteredResult != undefined && untrack(() => rerender())
	})
	$effect(() => {
		resolvedConfig.columnDefs
		untrack(() => updateTable(resolvedConfig))
	})
</script>

{#each Object.keys(components['tablecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.tablecomponent}
	/>
{/each}

<RunnableWrapper
	{outputs}
	{render}
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
>
	{#if Array.isArray(result) && result.every(isObject)}
		<div class="flex flex-col h-full">
			{#if resolvedConfig.search !== 'Disabled'}
				<div class="flex flex-row w-full justify-between items-center h-12">
					{#if resolvedConfig.search !== 'Disabled'}
						<div class="grow max-w-[300px]">
							<DebouncedInput
								placeholder="Search..."
								bind:value={searchValue}
								parentClass="h-full "
							/>
						</div>
					{:else}
						<div></div>
					{/if}

					{#if componentInput?.hideRefreshButton && componentInput['autoRefresh']}
						<RefreshButton {id} {loading} />
					{/if}
				</div>
			{:else if componentInput?.hideRefreshButton && componentInput['autoRefresh']}
				<div class="absolute right-2 top-2 z-50">
					<RefreshButton {id} {loading} />
				</div>
			{/if}
			<div
				class={twMerge(
					'component-wrapper bg-surface',
					'divide-y h-full',
					css?.container?.class ?? '',
					'wm-table-container',
					'flex flex-col'
				)}
				style={css?.container?.style ?? ''}
			>
				<div class="overflow-x-auto flex-1 w-full">
					<table class="relative w-full border-b">
						<thead
							class={twMerge(
								'bg-surface-secondary text-left',
								css?.tableHeader?.class ?? '',
								'wm-table-header',
								'sticky top-0 z-40'
							)}
							style={css?.tableHeader?.style ?? ''}
						>
							{#each getHeaderGroups($table) as headerGroup}
								<tr class="divide-x">
									{#each headerGroup.headers as header}
										{#if header?.column?.columnDef?.header}
											{@const context = header?.getContext()}
											{#if context}
												{@const component = renderCell(header.column.columnDef.header, context)}
												{@const displayName = getDisplayNameById(header.id)}
												<th class="!p-0">
													<div
														class="flex flex-row items-center gap-1 px-4 py-4 text-xs text-primary font-medium border-b"
													>
														{#if displayName}
															{displayName}
														{:else if !header.isPlaceholder && component}
															{@const SvelteComponent = component}
															<SvelteComponent />
														{/if}
														{#if header.column.getIsVisible()}
															<button
																class="w-6 flex justify-center items-center"
																onclick={() => {
																	header.column.toggleVisibility()
																}}
															>
																<EyeIcon
																	size="14"
																	class="hover:text-gray-600 text-gray-400 rounded-full "
																/>
															</button>
														{/if}
													</div>
												</th>
											{/if}
										{/if}
									{/each}
									{#if actionButtons.length > 0}
										<th class="!p-0">
											<div
												class="flex flex-row items-center px-4 py-4 text-xs text-primary font-medium border-b"
											>
												Actions
											</div>
										</th>
									{/if}
								</tr>
							{/each}
						</thead>
						<tbody
							class={twMerge('divide-y bg-surface', css?.tableBody?.class ?? '', 'wm-table-body')}
							style={css?.tableBody?.style ?? ''}
						>
							{#each $table.getRowModel().rows as row, index (row.id)}
								{@const isLastRow = index === filteredResult.length - 1}
								{@const rowIndex = row.original['__index']}
								<tr
									class={twMerge(
										isLastRow ? '!border-b-0' : '',
										'divide-x w-full',
										index % 2 === 0 ? 'bg-gray-50/50' : '',
										selectedRowIndex === rowIndex
											? 'bg-blue-100 hover:bg-blue-200 dark:bg-surface-selected dark:hover:bg-surface-hover divide-blue-200 hover:divide-blue-300 dark:divide-gray-600 dark:hover:divide-gray-700 wm-table-row-selected'
											: 'hover:bg-blue-50 dark:hover:bg-surface-hover wm-table-row'
									)}
								>
									{#each safeVisibleCell(row) as cell, index (index)}
										{#if cell?.column?.columnDef?.cell}
											{@const context = cell?.getContext()}
											{#if context}
												<AppCell
													on:keydown={() => toggleRow(row)}
													on:click={() => toggleRow(row)}
													type={resolvedConfig.columnDefs?.find(
														// TS types are wrong here
														// @ts-ignore
														(c) => c.field === cell.column.columnDef.accessorKey
													)?.type ?? 'text'}
													value={cell.getValue()}
													width={cell.column.getSize()}
													on:update={(event) => {
														updateCellValue(rowIndex, index, event.detail.value)
													}}
												/>
											{/if}
										{/if}
									{/each}

									{#if actionButtons.length > 0}
										<td
											class="p-2"
											onkeypress={() => toggleRow(row)}
											onclick={() => toggleRow(row)}
											style="width: {(actionButtons ?? []).length * 130}px"
										>
											<div class="center-center h-full w-full flex-wrap gap-2">
												{#each actionButtons as actionButton, actionIndex (actionButton?.id)}
													<!-- svelte-ignore a11y_no_static_element_interactions -->
													<RowWrapper
														value={row.original}
														index={rowIndex}
														onSet={(id, value) => {
															if (!inputs[id]) {
																inputs[id] = { [rowIndex]: value }
															} else {
																inputs[id] = { ...inputs[id], [rowIndex]: value }
															}

															outputs?.inputs.set(inputs, true)
														}}
														onRemove={(id) => {
															if (inputs?.[id] == undefined) {
																return
															}
															delete inputs[id][rowIndex]
															inputs[id] = { ...inputs[id] }
															if (Object.keys(inputs?.[id] ?? {}).length == 0) {
																delete inputs[id]
																inputs = { ...inputs }
															}
															outputs?.inputs.set(inputs, true)
														}}
													>
														<!-- svelte-ignore a11y_mouse_events_have_key_events -->
														<div
															onmouseover={stopPropagation(() => {
																if (actionButton.id !== $hoverStore) {
																	$hoverStore = actionButton.id
																}
															})}
															onmouseout={stopPropagation(() => {
																if ($hoverStore !== undefined) {
																	$hoverStore = undefined
																}
															})}
															class={classNames(
																($selectedComponent?.includes(actionButton.id) ||
																	$hoverStore === actionButton.id) &&
																	$mode !== 'preview'
																	? 'outline outline-indigo-500 outline-1 outline-offset-1 relative'
																	: 'relative'
															)}
														>
															{#if $mode !== 'preview'}
																<!-- svelte-ignore a11y_click_events_have_key_events -->
																<!-- svelte-ignore a11y_no_static_element_interactions -->
																<span
																	title={`Id: ${actionButton.id}`}
																	class={classNames(
																		'px-2 text-2xs font-bold w-fit absolute shadow  -top-2 -left-2 border z-50 rounded-sm',
																		'bg-indigo-500/90 border-indigo-600 text-white',
																		$selectedComponent?.includes(actionButton.id) ||
																			$hoverStore === actionButton.id
																			? 'opacity-100'
																			: 'opacity-0'
																	)}
																	onclick={stopPropagation(() => {
																		$selectedComponent = [actionButton.id]
																	})}
																>
																	{actionButton.id}
																</span>

																{#if $connectingInput.opened}
																	<div class="absolute z-50 left-8 -top-[10px]">
																		<Popover
																			floatingConfig={{
																				strategy: 'absolute',
																				placement: 'bottom-start'
																			}}
																			closeOnOtherPopoverOpen
																			contentClasses="p-4"
																		>
																			{#snippet trigger()}
																				<button
																					class="bg-red-500/70 border border-red-600 px-1 py-0.5"
																					title="Outputs"
																					aria-label="Open output"><Plug2 size={12} /></button
																				>
																			{/snippet}
																			{#snippet content()}
																				<ComponentOutputViewer
																					suffix="table"
																					on:select={({ detail }) =>
																						connectOutput(
																							connectingInput,
																							'buttoncomponent',
																							actionButton.id,
																							detail
																						)}
																					componentId={actionButton.id}
																				/>
																			{/snippet}
																		</Popover>
																	</div>
																{/if}
															{/if}
															{#if rowIndex == 0}
																{@const controls = {
																	left: () => {
																		if (actionIndex === 0) {
																			$selectedComponent = [id]
																			return true
																		} else if (actionIndex > 0) {
																			$selectedComponent = [actionButtons[actionIndex - 1].id]
																			return true
																		}
																		return false
																	},
																	right: () => {
																		if (actionIndex === actionButtons.length - 1) {
																			return id
																		} else if (actionIndex < actionButtons.length - 1) {
																			$selectedComponent = [actionButtons[actionIndex + 1].id]
																			return true
																		}
																		return false
																	}
																}}
																{#if actionButton.type == 'buttoncomponent'}
																	<AppButton
																		noInitialize
																		extraKey={'idx' + rowIndex}
																		{render}
																		noWFull
																		preclickAction={async () => {
																			toggleRow(row)
																		}}
																		id={actionButton.id}
																		customCss={actionButton.customCss}
																		configuration={actionButton.configuration}
																		recomputeIds={actionButton.recomputeIds}
																		extraQueryParams={{ row: row.original }}
																		componentInput={actionButton.componentInput}
																		{controls}
																	/>
																{:else if actionButton.type == 'checkboxcomponent'}
																	<AppCheckbox
																		noInitialize
																		extraKey={'idx' + rowIndex}
																		{render}
																		id={actionButton.id}
																		customCss={actionButton.customCss}
																		configuration={actionButton.configuration}
																		recomputeIds={actionButton.recomputeIds}
																		onToggle={actionButton.onToggle}
																		preclickAction={async () => {
																			toggleRow(row)
																		}}
																		{controls}
																	/>
																{:else if actionButton.type == 'selectcomponent'}
																	<div class="w-40">
																		<AppSelect
																			noDefault
																			noInitialize
																			extraKey={'idx' + rowIndex}
																			{render}
																			id={actionButton.id}
																			customCss={actionButton.customCss}
																			configuration={actionButton.configuration}
																			recomputeIds={actionButton.recomputeIds}
																			onSelect={actionButton.onSelect}
																			preclickAction={async () => {
																				toggleRow(row)
																			}}
																			{controls}
																		/>
																	</div>
																{/if}
															{:else if actionButton.type == 'buttoncomponent'}
																<AppButton
																	noInitialize
																	extraKey={'idx' + rowIndex}
																	{render}
																	noWFull
																	id={actionButton.id}
																	customCss={actionButton.customCss}
																	configuration={actionButton.configuration}
																	recomputeIds={actionButton.recomputeIds}
																	preclickAction={async () => {
																		toggleRow(row)
																	}}
																	extraQueryParams={{ row: row.original }}
																	componentInput={actionButton.componentInput}
																/>
															{:else if actionButton.type == 'checkboxcomponent'}
																<AppCheckbox
																	noInitialize
																	extraKey={'idx' + rowIndex}
																	{render}
																	id={actionButton.id}
																	customCss={actionButton.customCss}
																	configuration={actionButton.configuration}
																	recomputeIds={actionButton.recomputeIds}
																	onToggle={actionButton.onToggle}
																	preclickAction={async () => {
																		toggleRow(row)
																	}}
																/>
															{:else if actionButton.type == 'selectcomponent'}
																<div class="w-40">
																	<AppSelect
																		noDefault
																		noInitialize
																		extraKey={'idx' + rowIndex}
																		{render}
																		id={actionButton.id}
																		customCss={actionButton.customCss}
																		configuration={actionButton.configuration}
																		recomputeIds={actionButton.recomputeIds}
																		onSelect={actionButton.onSelect}
																		preclickAction={async () => {
																			toggleRow(row)
																		}}
																	/>
																</div>
															{/if}
														</div>
													</RowWrapper>
												{/each}
											</div>
										</td>
									{/if}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<AppTableFooter
					download={resolvedConfig?.downloadButton}
					pageSize={resolvedConfig?.pagination?.configuration?.auto?.pageSize ?? 20}
					manualPagination={resolvedConfig?.pagination?.selected == 'manual'}
					result={filteredResult}
					{table}
					class={twMerge(css?.tableFooter?.class, 'wm-table-footer')}
					style={css?.tableFooter?.style}
					{loading}
				/>
			</div>
		</div>
	{:else if result != undefined}
		<div class="flex flex-col h-full w-full overflow-auto">
			<Alert title="Parsing issues" type="error" size="xs" class="h-full w-full ">
				The result should be an array of objects. Received:
				<pre class="w-full bg-surface p-2 rounded-md whitespace-pre-wrap mt-2"
					>{JSON.stringify(result, null, 4)}
				</pre>
			</Alert>
		</div>
	{/if}
</RunnableWrapper>
