<script lang="ts" module>
	function validate(values: CreateTableValues, dbSchema?: DBSchema) {
		const columnNamesErrs = values.columns.flatMap((column) => {
			const isUnique = values.columns.filter((c) => c.name === column.name).length === 1
			return !column.name.length || !isUnique ? [column.name] : []
		})
		const fkErrs = values.foreignKeys.map((fk) => ({
			emptyTarget: !fk.targetTable,
			nonExistingSourceColumns: fk.columns
				.map((c) => c.sourceColumn)
				.filter((sc) => values.columns.every((c) => c.name !== sc)),
			nonExistingTargetColumns: fk.columns
				.map((c) => c.targetColumn)
				.filter(
					(tc) =>
						!tc ||
						!Object.keys(
							dbSchema?.schema?.[fk.targetTable?.split('.')?.[0] ?? '']?.[
								fk.targetTable?.split('.')[1]
							] ?? {}
						).includes(tc)
				)
		}))
		const someFkErr = fkErrs.some(
			(fkErr) =>
				fkErr.emptyTarget ||
				fkErr.nonExistingSourceColumns.length ||
				fkErr.nonExistingTargetColumns.length
		)

		const errs = {
			name: !values.name.length,
			columns: columnNamesErrs.length ? columnNamesErrs : undefined,
			foreignKeys: someFkErr ? fkErrs : undefined
		}
		if (Object.values(errs).every((v) => !v)) return undefined
		return errs
	}

	export type DBTableEditorProps = {
		onConfirm: (values: CreateTableValues) => void | Promise<void>
		previewSql?: (values: CreateTableValues) => string
		resourceType: DbType
		dbSchema?: DBSchema
		currentSchema?: string
	}
</script>

<script lang="ts">
	import { ArrowRight, ClipboardCopy, Info, Plus, Settings, X } from 'lucide-svelte'

	import { Button } from './common'
	import { Cell } from './table'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import {
		datatypeHasLength,
		dbSupportsSchemas,
		type DbType
	} from './apps/components/display/dbtable/utils'
	import { DB_TYPES } from '$lib/consts'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import {
		datatypeDefaultLength,
		type CreateTableValues
	} from './apps/components/display/dbtable/queries/createTable'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard } from '$lib/utils'
	import { getFlatTableNamesFromSchema, type DBSchema } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'

	const { onConfirm, resourceType, previewSql, dbSchema, currentSchema }: DBTableEditorProps =
		$props()

	const columnTypes = DB_TYPES[resourceType]
	const defaultColumnType = (
		{
			postgresql: 'VARCHAR',
			snowflake: 'varchar',
			ms_sql_server: 'varchar',
			bigquery: 'string',
			mysql: 'varchar'
		} satisfies Record<DbType, string>
	)[resourceType]

	const values: CreateTableValues = $state({
		name: '',
		columns: [],
		foreignKeys: []
	})

	function addColumn({ name, primaryKey }: { name: string; primaryKey?: boolean }) {
		values.columns.push({
			name,
			datatype: defaultColumnType,
			...(datatypeHasLength(defaultColumnType) && {
				datatype_length: datatypeDefaultLength(defaultColumnType)
			}),
			...(primaryKey && { primaryKey })
		})
	}
	addColumn({ name: 'id', primaryKey: true })

	const errors: ReturnType<typeof validate> = $derived(validate(values, dbSchema))

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & { onConfirm: () => void; codeContent?: string })
		| undefined = $state()
</script>

<div class="flex flex-col h-full">
	<div class="flex-1 overflow-y-auto flex flex-col gap-6">
		<label>
			Name
			<input
				type="text"
				placeholder="my_table"
				class={errors?.name ? 'border !border-red-600/60' : ''}
				bind:value={values.name}
			/>
		</label>

		<div class="flex flex-col">
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<label>Columns</label>
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Type</Cell>
						<Cell head last>Primary</Cell>
					</tr>
				</Head>
				<tbody class="divide-y bg-surface">
					{#each values.columns as column, i}
						<tr>
							<Cell first>
								<input
									type="text"
									class={'h-10 ' +
										(errors?.columns?.includes(column.name) ? 'border !border-red-600/60' : '')}
									style="height: 2rem;"
									placeholder="column_name"
									bind:value={column.name}
								/>
							</Cell>
							<Cell>
								<Select
									containerStyles="--height: 2rem; --font-size: 14px;"
									class="!w-48"
									value={column.datatype}
									on:change={(e) => {
										column.datatype = e.detail.value
										if (datatypeHasLength(column.datatype)) {
											column.datatype_length = datatypeDefaultLength(column.datatype)
										} else {
											column.datatype_length = undefined
										}
									}}
									items={columnTypes}
									clearable={false}
								/>
							</Cell>
							<Cell last class="flex items-center mt-1.5">
								<input type="checkbox" class="!w-4 !h-4" bind:checked={column.primaryKey} />
								<Popover class="ml-8" contentClasses="py-3 px-5 flex flex-col gap-6">
									{#snippet trigger()}
										<Settings size={18} />
									{/snippet}
									{#snippet content()}
										{#if datatypeHasLength(column.datatype)}
											<label class="text-xs">
												Length
												<input type="number" placeholder="0" bind:value={column.datatype_length} />
											</label>
										{/if}
										<label class="text-xs">
											<span class="flex gap-1 mb-1">
												Default value
												<Tooltip>
													<Info size={14} />
													{#snippet text()}
														Surrender your expressions with curly brackets:
														<code>
															{'{NOW()}'}
														</code>.
														<br />
														By default, it will be parsed as a literal
													{/snippet}
												</Tooltip>
											</span>
											<input type="text" placeholder="NULL" bind:value={column.defaultValue} />
										</label>
										{#if !column.primaryKey}
											<label class="flex gap-2 items-center text-xs">
												<input type="checkbox" class="!w-4 !h-4" bind:checked={column.not_null} />
												Not nullable
											</label>
										{/if}
									{/snippet}
								</Popover>
								<Button
									color="light"
									startIcon={{ icon: X }}
									wrapperClasses="w-fit ml-2"
									btnClasses="p-0"
									on:click={() => values.columns.splice(i, 1)}
								/>
							</Cell>
						</tr>
					{/each}
					<tr class="w-full">
						<td colspan={99} class="p-1">
							<Button
								wrapperClasses="mx-auto"
								startIcon={{ icon: Plus }}
								color="light"
								on:click={() => addColumn({ name: '' })}
							>
								Add
							</Button>
						</td>
					</tr>
				</tbody>
			</DataTable>
		</div>
		<div class="flex flex-col">
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<label>Foreign Keys</label>
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Table</Cell>
						<Cell head last>Columns</Cell>
					</tr>
				</Head>
				<tbody class="divide-y bg-surface">
					{#each values.foreignKeys as foreignKey, foreignKeyIndex}
						{@const fkErrors = errors?.foreignKeys?.[foreignKeyIndex]}
						<tr>
							<Cell first class="flex">
								<Select
									containerStyles="--height: 2rem; --font-size: 14px;"
									class={twMerge('!w-48', fkErrors?.emptyTarget ? 'border !border-red-600/60' : '')}
									placeholder=""
									value={foreignKey.targetTable}
									on:change={(e) => (foreignKey.targetTable = e.detail.value)}
									items={getFlatTableNamesFromSchema(dbSchema).map((o) => ({
										value: o,
										label:
											(currentSchema && o.startsWith(currentSchema)) ||
											!dbSupportsSchemas(resourceType)
												? o.split('.')[1]
												: o
									}))}
									clearable={false}
								/>
							</Cell>
							<Cell>
								<div class="flex flex-col gap-2">
									{#each foreignKey.columns as column, columnIndex}
										<div class="flex">
											<div class="flex items-center gap-1 w-60">
												<!-- Div wrappers with absolute select are to prevent the Select content
												 		 from overflowing -->
												<div class="grow h-[2rem] relative">
													<Select
														containerStyles="--height: 2rem; --font-size: 14px;"
														class={twMerge(
															'!absolute inset-0',
															fkErrors?.nonExistingSourceColumns.includes(column.sourceColumn)
																? 'border !border-red-600/60'
																: ''
														)}
														placeholder=""
														value={column.sourceColumn}
														on:change={(e) => (column.sourceColumn = e.detail.value)}
														items={values.columns.map((c) => c.name)}
														clearable={false}
													/>
												</div>
												<ArrowRight size={16} class="h-fit shrink-0" />
												<div class="grow h-[2rem] relative">
													<Select
														containerStyles="--height: 2rem; --font-size: 14px;"
														class={twMerge(
															'!absolute inset-0',
															fkErrors?.nonExistingTargetColumns.includes(column.targetColumn)
																? 'border !border-red-600/60'
																: ''
														)}
														placeholder=""
														value={column.targetColumn}
														on:change={(e) => (column.targetColumn = e.detail.value)}
														items={Object.keys(
															dbSchema?.schema?.[foreignKey.targetTable?.split('.')?.[0] ?? '']?.[
																foreignKey.targetTable?.split('.')[1]
															] ?? {}
														)}
														clearable={false}
													/>
												</div>
											</div>
											<div class="ml-auto flex">
												{#if columnIndex === 0}
													<Popover contentClasses="py-3 px-5 w-52 flex flex-col gap-6">
														{#snippet trigger()}
															<Settings size={18} />
														{/snippet}
														{#snippet content()}
															<span>
																ON DELETE <select bind:value={foreignKey.onDelete}>
																	<option value="NO ACTION" selected>NO ACTION</option>
																	<option value="CASCADE" selected>CASCADE</option>
																	<option value="SET NULL" selected>SET NULL</option>
																</select>
															</span>
															<span>
																ON UPDATE <select bind:value={foreignKey.onUpdate}>
																	<option value="NO ACTION" selected>NO ACTION</option>
																	<option value="CASCADE" selected>CASCADE</option>
																	<option value="SET NULL" selected>SET NULL</option>
																</select>
															</span>
														{/snippet}
													</Popover>
												{/if}
												<Button
													color="light"
													startIcon={{ icon: X }}
													wrapperClasses="w-fit ml-2"
													btnClasses="p-0"
													on:click={foreignKey.columns.length > 1
														? () => foreignKey.columns.splice(columnIndex, 1)
														: () => values.foreignKeys.splice(foreignKeyIndex, 1)}
												/>
											</div>
										</div>
									{/each}
									<button
										class="w-60 border-dashed border-2 rounded-md flex justify-center items-center py-1 gap-2 text-gray-500 font-normal"
										onclick={() => foreignKey.columns.push({})}
									>
										<Plus class="h-fit" size={12} /> Add
									</button>
								</div></Cell
							>
						</tr>
					{/each}
					<tr class="w-full">
						<td colspan={99} class="p-1">
							<Button
								wrapperClasses="mx-auto"
								startIcon={{ icon: Plus }}
								color="light"
								on:click={() =>
									values.foreignKeys.push({
										columns: [{}],
										onDelete: 'NO ACTION',
										onUpdate: 'NO ACTION'
									})}
							>
								Add
							</Button>
						</td>
					</tr>
				</tbody>
			</DataTable>
		</div>
	</div>
	<Button
		disabled={!!errors}
		on:click={() =>
			(askingForConfirmation = {
				onConfirm: async () => {
					try {
						askingForConfirmation && (askingForConfirmation.loading = true)
						await onConfirm(values)
						sendUserToast(values.name + ' created!')
					} catch (e) {
						let msg: string | undefined = (e as Error)?.message
						if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : 'An error occured'
						sendUserToast(msg, true)
					}
					askingForConfirmation = undefined
				},
				title: 'Confirm running the following:',
				confirmationText: 'Create ' + values.name,
				open: true,
				...(previewSql && { codeContent: previewSql(values) })
			})}>Create table</Button
	>
</div>

<ConfirmationModal
	{...askingForConfirmation ?? { confirmationText: '', title: '' }}
	on:canceled={() => (askingForConfirmation = undefined)}
	on:confirmed={askingForConfirmation?.onConfirm ?? (() => {})}
>
	{#if askingForConfirmation?.codeContent}
		<div class="bg-surface-secondary border border-surface-selected rounded-md p-2 relative">
			<code class="whitespace-pre-wrap">
				{askingForConfirmation.codeContent}
			</code>
			<Button
				on:click={() => copyToClipboard(askingForConfirmation?.codeContent)}
				size="xs"
				startIcon={{ icon: ClipboardCopy }}
				color="none"
				wrapperClasses="absolute z-10 top-0 right-0"
			></Button>
		</div>
	{/if}
</ConfirmationModal>
