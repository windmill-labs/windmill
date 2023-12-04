<script lang="ts">
	import { getContext, onDestroy, onMount } from 'svelte'
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
	import {
		createSvelteTable,
		flexRender,
		type HeaderGroup,
		type Row,
		type Table,
		type TableOptions
	} from '@tanstack/svelte-table'
	import AppButton from '../../buttons/AppButton.svelte'
	import { classNames, isObject, sendUserToast } from '$lib/utils'
	import DebouncedInput from '../../helpers/DebouncedInput.svelte'
	import AppTableFooter from './AppTableFooter.svelte'
	import { tableOptions } from './tableOptions'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { components, type ButtonComponent } from '../../../editor/component'
	import { initCss } from '../../../utils'
	import { twMerge } from 'tailwind-merge'
	import { connectOutput, initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import AppCheckbox from '../../inputs/AppCheckbox.svelte'
	import AppSelect from '../../inputs/AppSelect.svelte'
	import RowWrapper from '../../layout/RowWrapper.svelte'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import { Popup } from '$lib/components/common'
	import ComponentOutputViewer from '$lib/components/apps/editor/contextPanel/ComponentOutputViewer.svelte'
	import { Plug2 } from 'lucide-svelte'
	import AppCell from './AppCell.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	type T = Record<string, any>

	let result: Record<string, any>[] | undefined = undefined

	const {
		app,
		worldStore,
		componentControl,
		selectedComponent,
		hoverStore,
		mode,
		connectingInput
	} = getContext<AppViewerContext>('AppViewerContext')

	let searchValue = ''

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: undefined,
		loading: false,
		result: [],
		inputs: {},
		search: '',
		page: 1
	})

	let inputs = {}
	let loading: boolean = false

	$: setSearch(searchValue)

	function setSearch(srch: string) {
		outputs?.search?.set(srch)
	}

	const options = writable<TableOptions<T>>({
		...tableOptions,
		data: [],
		columns: []
	})

	let table = createSvelteTable(options)

	let resolvedConfig = initConfig(
		components['tablecomponent'].initialData.configuration,
		configuration
	)

	let selectedRowIndex = -1

	function toggleRow(row: Record<string, any>, force: boolean = false) {
		let data = { ...row.original }
		let index = data['__index']
		delete data['__index']
		if (
			selectedRowIndex !== index ||
			JSON.stringify(data) !== JSON.stringify(result?.[index]) ||
			force
		) {
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

	let mounted = false
	onMount(() => {
		mounted = true
	})

	$: selectedRowIndex === -1 &&
		Array.isArray(result) &&
		result.length > 0 &&
		// We need to wait until the component is mounted so the world is created
		mounted &&
		outputs &&
		toggleRow({ original: { ...result[0], __index: 0 } })

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value?.toString()?.includes(searchValue))
		)
	}

	function renderCell(x: any, props: any): any {
		try {
			return flexRender(x, props)
		} catch (e) {
			return undefined
		}
	}

	let filteredResult: Array<Record<string, any>> = []

	function setFilteredResult() {
		const wIndex = Array.isArray(result)
			? (result as any[]).map((x, i) => ({ ...x, __index: i }))
			: [{ error: 'input was not an array' }]

		if (resolvedConfig.search === 'By Runnable' || resolvedConfig.search === 'Disabled') {
			filteredResult = wIndex ?? []
		} else {
			filteredResult = searchInResult(wIndex ?? [], searchValue)
		}
	}
	$: (result || resolvedConfig.search || searchValue || resolvedConfig.pagination) &&
		setFilteredResult()

	$: outputs.page.set($table.getState().pagination.pageIndex)

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
					? resolvedConfig?.pagination?.configuration.manual.pageCount ?? -1
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
			toggleRow({ original: filteredResult[0] }, true)
		}

		if (outputs.page.peak()) {
			$table.setPageIndex(outputs?.page.peak())
		}
	}

	$: filteredResult != undefined && rerender()

	let css = initCss($app.css?.tablecomponent, customCss)

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

	function updateTable(resolvedConfig, searchValue) {
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

			$table.setColumnOrder(() =>
				resolvedConfig.columnDefs.map((columnDef: { field: any }) => columnDef.field)
			)
		}
	}

	$: $table && updateTable(resolvedConfig, searchValue)
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
		<div
			class={twMerge(
				'border  shadow-sm divide-y h-full',
				css?.container?.class ?? '',
				'wm-table-container',
				'flex flex-col'
			)}
			style={css?.container?.style ?? ''}
		>
			{#if resolvedConfig.search !== 'Disabled'}
				<div class="px-2 py-1">
					<div class="flex items-center">
						<div class="grow max-w-[300px]">
							<DebouncedInput placeholder="Search..." bind:value={searchValue} />
						</div>
					</div>
				</div>
			{/if}

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
												<span class="block px-4 py-4 text-sm font-semibold border-b">
													{#if displayName}
														{displayName}
													{:else if !header.isPlaceholder && component}
														<svelte:component this={component} />
													{/if}
												</span>
											</th>
										{/if}
									{/if}
								{/each}
								{#if actionButtons.length > 0}
									<th class="!p-0">
										<span class="block px-4 py-4 text-sm font-semibold border-b"> Actions </span>
									</th>
								{/if}
							</tr>
						{/each}
					</thead>
					<tbody
						class={twMerge('divide-y bg-surface', css?.tableBody?.class ?? '', 'wm-table-body')}
						style={css?.tableBody?.style ?? ''}
					>
						{#each $table.getRowModel().rows as row (row.id)}
							{@const rowIndex = row.original['__index']}
							<tr
								class={classNames(
									'last-of-type:!border-b-0 divide-x w-full',
									selectedRowIndex === rowIndex
										? 'bg-blue-100 hover:bg-blue-200 dark:bg-surface-selected dark:hover:bg-surface-hover divide-blue-200 hover:divide-blue-300 dark:divide-gray-600 dark:hover:divide-gray-700 wm-table-row-selected'
										: 'hover:bg-blue-50 dark:hover:bg-surface-hover wm-table-row'
								)}
							>
								{#each safeVisibleCell(row) as cell, index (index)}
									{#if cell?.column?.columnDef?.cell}
										{@const context = cell?.getContext()}
										{#if context}
											<td
												on:keydown={() => toggleRow(row)}
												on:click={() => toggleRow(row)}
												class="p-4 whitespace-pre-wrap truncate text-xs text-primary"
												style={'width: ' + cell.column.getSize() + 'px'}
											>
												<AppCell
													type={resolvedConfig.columnDefs?.find(
														// TS types are wrong here
														// @ts-ignore
														(c) => c.field === cell.column.columnDef.accessorKey
													)?.type ?? 'text'}
													value={cell.getValue()}
												/>
											</td>
										{/if}
									{/if}
								{/each}

								{#if actionButtons.length > 0}
									<td
										class="p-2"
										on:keypress={() => toggleRow(row)}
										on:click={() => toggleRow(row)}
										style="width: {(actionButtons ?? []).length * 130}px"
									>
										<div class="center-center h-full w-full flex-wrap gap-2">
											{#each actionButtons as actionButton, actionIndex (actionButton?.id)}
												<!-- svelte-ignore a11y-no-static-element-interactions -->
												<RowWrapper
													value={row.original}
													index={rowIndex}
													on:set={(e) => {
														const { id, value } = e.detail
														if (!inputs[id]) {
															inputs[id] = { [rowIndex]: value }
														} else {
															inputs[id] = { ...inputs[id], [rowIndex]: value }
														}
														outputs?.inputs.set(inputs, true)
													}}
													on:remove={(e) => {
														const id = e.detail
														if (inputs?.[id] == undefined) {
															return
														}
														if (rowIndex == 0) {
															delete inputs[id]
															inputs = { ...inputs }
														} else {
															delete inputs[id][rowIndex]
															inputs[id] = { ...inputs[id] }
														}
														outputs?.inputs.set(inputs, true)
													}}
												>
													<!-- svelte-ignore a11y-mouse-events-have-key-events -->
													<div
														on:mouseover|stopPropagation={() => {
															if (actionButton.id !== $hoverStore) {
																$hoverStore = actionButton.id
															}
														}}
														on:mouseout|stopPropagation={() => {
															if ($hoverStore !== undefined) {
																$hoverStore = undefined
															}
														}}
														class={classNames(
															($selectedComponent?.includes(actionButton.id) ||
																$hoverStore === actionButton.id) &&
																$mode !== 'preview'
																? 'outline outline-indigo-500 outline-1 outline-offset-1 relative'
																: 'relative'
														)}
													>
														{#if $mode !== 'preview'}
															<!-- svelte-ignore a11y-click-events-have-key-events -->
															<!-- svelte-ignore a11y-no-static-element-interactions -->
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
																on:click|stopPropagation={() => {
																	$selectedComponent = [actionButton.id]
																}}
															>
																{actionButton.id}
															</span>

															{#if $connectingInput.opened}
																<div class="absolute z-50 left-8 -top-[10px]">
																	<Popup
																		floatingConfig={{
																			strategy: 'absolute',
																			placement: 'bottom-start'
																		}}
																	>
																		<svelte:fragment slot="button">
																			<button
																				class="bg-red-500/70 border border-red-600 px-1 py-0.5"
																				title="Outputs"
																				aria-label="Open output"><Plug2 size={12} /></button
																			>
																		</svelte:fragment>
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
																	</Popup>
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
																	extraKey={'idx' + rowIndex}
																	{render}
																	id={actionButton.id}
																	customCss={actionButton.customCss}
																	configuration={actionButton.configuration}
																	recomputeIds={actionButton.recomputeIds}
																	preclickAction={async () => {
																		toggleRow(row)
																	}}
																	{controls}
																/>
															{:else if actionButton.type == 'selectcomponent'}
																<div class="w-40">
																	<AppSelect
																		extraKey={'idx' + rowIndex}
																		{render}
																		id={actionButton.id}
																		customCss={actionButton.customCss}
																		configuration={actionButton.configuration}
																		recomputeIds={actionButton.recomputeIds}
																		preclickAction={async () => {
																			toggleRow(row)
																		}}
																		{controls}
																	/>
																</div>
															{/if}
														{:else if actionButton.type == 'buttoncomponent'}
															<AppButton
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
																extraKey={'idx' + rowIndex}
																{render}
																id={actionButton.id}
																customCss={actionButton.customCss}
																configuration={actionButton.configuration}
																recomputeIds={actionButton.recomputeIds}
																preclickAction={async () => {
																	toggleRow(row)
																}}
															/>
														{:else if actionButton.type == 'selectcomponent'}
															<div class="w-40">
																<AppSelect
																	--font-size="10px"
																	extraKey={'idx' + rowIndex}
																	{render}
																	id={actionButton.id}
																	customCss={actionButton.customCss}
																	configuration={actionButton.configuration}
																	recomputeIds={actionButton.recomputeIds}
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
