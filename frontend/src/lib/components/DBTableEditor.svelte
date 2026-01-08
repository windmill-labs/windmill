<script lang="ts" module>
	function validate(values: TableEditorValues, dbSchema?: DBSchema) {
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
</script>

<script lang="ts">
	import { ArrowRight, ClipboardCopy, Plus, Settings, X } from 'lucide-svelte'

	import { Button } from './common'
	import { Cell } from './table'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import { datatypeHasLength } from './apps/components/display/dbtable/utils'
	import { DB_TYPES } from '$lib/consts'
	import Popover from './meltComponents/Popover.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard } from '$lib/utils'
	import { getFlatTableNamesFromSchema, type DBSchema } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import type { DbType } from './dbTypes'
	import Portal from './Portal.svelte'
	import { Debounced } from 'runed'
	import {
		type TableEditorValues,
		datatypeDefaultLength
	} from './apps/components/display/dbtable/tableEditor'
	import Alert from './common/alert/Alert.svelte'
	import type { DbFeatures } from './apps/components/display/dbtable/dbFeatures'
	import Label from './Label.svelte'

	type Props = {
		dbType: DbType
		dbSchema?: DBSchema
		currentSchema?: string
		initialValues?: TableEditorValues
		features?: DbFeatures
		onConfirm: (params: { values: TableEditorValues }) => void | Promise<void>
		computePreview: (params: {
			values: TableEditorValues
		}) => { sql: string; alert?: { title: string; body?: string } }
		computeBtnProps: (params: { values: TableEditorValues }) => { text: string; disabled?: boolean }
	}

	const {
		dbType,
		dbSchema,
		currentSchema,
		initialValues,
		features,
		onConfirm,
		computeBtnProps,
		computePreview
	}: Props = $props()

	const columnTypes = DB_TYPES[dbType]
	const defaultColumnType = (
		{
			postgresql: 'BIGSERIAL',
			snowflake: 'varchar',
			ms_sql_server: 'varchar',
			bigquery: 'string',
			mysql: 'varchar',
			duckdb: 'string'
		} satisfies Record<DbType, string>
	)[dbType]

	const values: TableEditorValues = $state(
		$state.snapshot(initialValues) ?? {
			name: '',
			columns: [],
			foreignKeys: []
		}
	)

	function addColumn({ name, primaryKey }: { name: string; primaryKey?: boolean }) {
		values.columns.push({
			name,
			datatype: defaultColumnType,
			...(datatypeHasLength(defaultColumnType) && {
				datatype_length: datatypeDefaultLength(defaultColumnType)
			}),
			...(primaryKey && { primaryKey }),
			...(!features?.defaultToNotNull && { nullable: true })
		})
	}
	if (!initialValues) {
		addColumn({ name: 'id', primaryKey: features?.primaryKeys })
	}

	const errors: ReturnType<typeof validate> = $derived(validate(values, dbSchema))

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & {
				onConfirm: () => void
				codeContent?: string
				alert?: { title: string; body?: string }
		  })
		| undefined = $state()

	let darkMode = $state(false)

	let btnProps = new Debounced(() => computeBtnProps({ values }), 500)
</script>

<DarkModeObserver bind:darkMode />

<div class="flex flex-col h-full">
	<div class="flex-1 overflow-y-auto flex flex-col gap-6">
		<label>
			Name
			<TextInput
				inputProps={{ type: 'text', placeholder: 'my_table' }}
				error={errors?.name}
				bind:value={values.name}
			/>
		</label>

		<div class="flex flex-col" id="columns-section">
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<label>Columns</label>
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Type</Cell>
						<Cell head last>{features?.primaryKeys ? 'Primary' : ''}</Cell>
					</tr>
				</Head>
				<tbody class="divide-y bg-surface">
					{#each values.columns as column, i}
						<tr>
							<Cell first>
								<div class="flex flex-col">
									<TextInput
										error={errors?.columns?.includes(column.name)}
										inputProps={{ type: 'text', placeholder: 'column_name' }}
										bind:value={
											() => column.name,
											(newName) => {
												const oldName = column.name
												column.name = newName
												// Update all foreign keys that reference this column
												if (oldName && oldName !== newName) {
													values.foreignKeys.forEach((fk) => {
														fk.columns.forEach((fkCol) => {
															if (fkCol.sourceColumn === oldName) {
																fkCol.sourceColumn = newName
															}
														})
													})
												}
											}
										}
									/>
									{#if column.initialName && column.name !== column.initialName}
										<span class="text-xs text-hint">Old name: {column.initialName}</span>
									{/if}
								</div>
							</Cell>
							<Cell>
								<Select
									bind:value={
										() => column.datatype,
										(v) => {
											column.datatype = v
											if (datatypeHasLength(column.datatype)) {
												column.datatype_length = datatypeDefaultLength(column.datatype)
											} else {
												column.datatype_length = undefined
											}
										}
									}
									items={safeSelectItems(columnTypes)}
									class="w-48"
								/>
								{#if column.initialName && column.name !== column.initialName}
									<span class="text-xs">&nbsp;</span>
								{/if}
							</Cell>
							<Cell last class="flex items-center mt-1.5">
								{#if features?.primaryKeys}
									<input
										type="checkbox"
										class="!w-4 !h-4 primary-key-checkbox"
										bind:checked={column.primaryKey}
									/>
								{/if}
								<Popover
									class="ml-8"
									contentClasses="py-3 px-5 flex flex-col gap-6"
									enableFlyTransition
								>
									{#snippet trigger()}
										<Settings size={18} class="settings-menu-btn" />
									{/snippet}
									{#snippet content()}
										{#if datatypeHasLength(column.datatype)}
											<label class="text-xs">
												Length
												<input type="number" placeholder="0" bind:value={column.datatype_length} />
											</label>
										{/if}
										{#if features?.defaultValues}
											<Label
												class="flex gap-1 mb-1"
												label="Default Value"
												tooltip="Parsed as literal by default. Use curly brackets for expressions (e.g. {'{NOW()}'} )."
											>
												<input
													class="default-value"
													type="text"
													placeholder="NULL"
													bind:value={column.defaultValue}
												/>
											</Label>
										{/if}
										{#if !column.primaryKey}
											<label class="flex gap-2 items-center text-xs">
												<input
													type="checkbox"
													class="nullable-checkbox !w-4 !h-4"
													bind:checked={column.nullable}
												/>
												Nullable
											</label>
										{/if}
									{/snippet}
								</Popover>
								<Button
									color="light"
									startIcon={{ icon: X }}
									wrapperClasses="w-fit ml-2"
									btnClasses="delete-column-btn p-0"
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
		{#if features?.foreignKeys}
			<div class="flex flex-col" id="foreign-keys-section">
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
										inputClass={twMerge('fk-table-select !w-48')}
										error={fkErrors?.emptyTarget}
										placeholder=""
										bind:value={foreignKey.targetTable}
										items={getFlatTableNamesFromSchema(dbSchema).map((o) => ({
											value: o,
											label:
												(currentSchema && o.startsWith(currentSchema)) || !features?.schemas
													? o.split('.')[1]
													: o
										}))}
									/>
								</Cell>
								<Cell>
									<div class="flex flex-col gap-2">
										{#each foreignKey.columns as column, columnIndex}
											<div class="flex">
												<div class="flex items-center gap-1 w-60">
													<!-- Div wrappers with absolute select are to prevent the Select content
												 		 from x-overflowing -->
													<div class="grow h-[2rem] relative">
														<Select
															class="fk-source-col-select !absolute inset-0"
															error={fkErrors?.nonExistingSourceColumns.includes(
																column.sourceColumn
															)}
															placeholder=""
															bind:value={column.sourceColumn}
															items={values.columns
																.filter((c) => c.name.length)
																.map((c) => ({ value: c.name }))}
															clearable={false}
														/>
													</div>
													<ArrowRight size={16} class="h-fit shrink-0" />
													<div class="grow h-[2rem] relative">
														<Select
															class="fk-target-col-select !absolute inset-0"
															error={fkErrors?.nonExistingTargetColumns.includes(
																column.targetColumn
															)}
															placeholder=""
															bind:value={column.targetColumn}
															items={Object.keys(
																dbSchema?.schema?.[foreignKey.targetTable?.split('.')?.[0] ?? '']?.[
																	foreignKey.targetTable?.split('.')[1]
																] ?? {}
															).map((value) => ({ value }))}
															clearable={false}
														/>
													</div>
												</div>
												<div class="ml-auto flex">
													{#if columnIndex === 0}
														<Popover
															contentClasses="py-3 px-5 w-52 flex flex-col gap-4"
															enableFlyTransition
														>
															{#snippet trigger()}
																<Settings class="fk-settings-btn" size={18} />
															{/snippet}
															{#snippet content()}
																<Label label="On delete">
																	<select
																		class="fk-on-delete-select"
																		bind:value={foreignKey.onDelete}
																	>
																		<option value="NO ACTION" selected>No action</option>
																		<option value="CASCADE" selected>Cascade</option>
																		<option value="SET NULL" selected>Set null</option>
																	</select>
																</Label>
																<Label label="On update">
																	<select
																		class="fk-on-update-select"
																		bind:value={foreignKey.onUpdate}
																	>
																		<option value="NO ACTION" selected>No action</option>
																		<option value="CASCADE" selected>Cascade</option>
																		<option value="SET NULL" selected>Set null</option>
																	</select>
																</Label>
															{/snippet}
														</Popover>
													{/if}
													<Button
														color="light"
														startIcon={{ icon: X }}
														wrapperClasses="w-fit ml-2"
														btnClasses="fk-delete-btn p-0"
														on:click={foreignKey.columns.length > 1
															? () => foreignKey.columns.splice(columnIndex, 1)
															: () => values.foreignKeys.splice(foreignKeyIndex, 1)}
													/>
												</div>
											</div>
										{/each}
										<button
											class="w-60 border-dashed dark:border-gray-600 border-2 rounded-md flex justify-center items-center py-1 gap-2 text-primary-500 font-normal"
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
		{/if}
	</div>
	<Button
		disabled={!!errors || btnProps.current.disabled}
		loading={btnProps.pending}
		on:click={() => {
			let preview = computePreview?.({ values })
			askingForConfirmation = {
				onConfirm: async () => {
					try {
						askingForConfirmation && (askingForConfirmation.loading = true)
						await onConfirm({ values })
					} catch (e) {
						let msg: string | undefined = (e as Error)?.message
						if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : 'An error occurred'
						sendUserToast(msg, true)
					}
					askingForConfirmation = undefined
				},
				title: 'Confirm running the following:',
				confirmationText: btnProps.current.text,
				open: true,
				...(preview && { codeContent: preview.sql, alert: preview.alert })
			}
		}}
	>
		{btnProps.current.text}
	</Button>
</div>

<Portal>
	<ConfirmationModal
		id="db-table-editor-confirmation-modal"
		{...askingForConfirmation ?? { confirmationText: '', title: '' }}
		on:canceled={() => (askingForConfirmation = undefined)}
		on:confirmed={askingForConfirmation?.onConfirm ?? (() => {})}
	>
		{#if askingForConfirmation?.alert}
			<Alert title={askingForConfirmation.alert.title} type="error" class="mb-2">
				{askingForConfirmation.alert.body}
			</Alert>
		{/if}
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
</Portal>
