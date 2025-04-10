<script lang="ts" module>
	function validate(values: Values) {
		const columnNamesErrs = values.columns.flatMap((column) => {
			const isUnique = values.columns.filter((c) => c.name === column.name).length === 1
			return !column.name.length || !isUnique ? [column.name] : []
		})
		const errs = {
			name: !values.name.length,
			columns: columnNamesErrs.length ? columnNamesErrs : undefined
		}
		if (Object.values(errs).every((v) => !v)) return undefined
		return errs
	}

	export type DBTableEditorProps = {
		onConfirm: (params: {}) => void | Promise<void>
		resourceType: DbType
	}

	type Values = {
		name: string
		columns: {
			name: string
			type: string
			primaryKey?: boolean
			defaultValue?: string
			not_null?: boolean
		}[]
	}
</script>

<script lang="ts">
	import { Info, Plus, Settings, X } from 'lucide-svelte'

	import { Button } from './common'
	import { Cell } from './table'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import type { DbType } from './apps/components/display/dbtable/utils'
	import { DB_TYPES } from '$lib/consts'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'

	const { onConfirm, resourceType }: DBTableEditorProps = $props()

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

	const values: Values = $state({
		name: '',
		columns: [{ name: 'id', type: defaultColumnType, primaryKey: true }]
	})

	const errors: ReturnType<typeof validate> = $derived(validate(values))
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

		<div>
			<!-- svelte-ignore a11y_label_has_associated_control -->
			<label>Columns</label>
			<div>
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
										placeholder="column_name"
										bind:value={column.name}
									/>
								</Cell>
								<Cell>
									<Select
										class="!w-48"
										value={column.type}
										on:change={(e) => (column.type = e.detail.value)}
										items={columnTypes}
										clearable={false}
									/>
								</Cell>
								<Cell last class="flex items-center mt-3">
									<input type="checkbox" class="!w-4 !h-4" bind:checked={column.primaryKey} />
									<Popover class="ml-8" contentClasses="py-3 px-5 flex flex-col gap-6">
										{#snippet trigger()}
											<Settings size={18} />
										{/snippet}
										{#snippet content()}
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
											<label class="flex gap-2 items-center text-xs">
												<input type="checkbox" class="!w-4 !h-4" bind:checked={column.not_null} />
												Not nullable
											</label>
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
									on:click={() =>
										values.columns.push({ name: '', type: defaultColumnType, primaryKey: false })}
								>
									Add
								</Button>
							</td>
						</tr>
					</tbody>
				</DataTable>
			</div>
		</div>
	</div>
	<Button disabled={!!errors} on:click={() => onConfirm(values)}>Create table</Button>
</div>
