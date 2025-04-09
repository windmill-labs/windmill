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
		}[]
	}
</script>

<script lang="ts">
	import { Plus, X } from 'lucide-svelte'

	import { Button } from './common'
	import { Cell } from './table'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import type { DbType } from './apps/components/display/dbtable/utils'
	import { DB_TYPES } from '$lib/consts'

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
			<span class="flex items-center justify-between">
				<!-- svelte-ignore a11y_label_has_associated_control -->
				<label>Columns</label>
				<Button
					wrapperClasses="w-fit pb-1"
					startIcon={{ icon: Plus }}
					color="light"
					on:click={() =>
						values.columns.push({ name: '', type: defaultColumnType, primaryKey: false })}
					>Add</Button
				>
			</span>
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
									class={errors?.columns?.includes(column.name) ? 'border !border-red-600/60' : ''}
									placeholder="column_name"
									bind:value={column.name}
								/>
							</Cell>
							<Cell>
								<select bind:value={column.type}>
									{#each columnTypes as columnType}
										<option value={columnType}>{columnType}</option>
									{/each}
								</select>
							</Cell>
							<Cell last class="flex items-center mt-2">
								<input type="checkbox" class="!w-4 !h-4" bind:checked={column.primaryKey} />
								<Button
									color="light"
									startIcon={{ icon: X }}
									wrapperClasses="w-fit ml-auto"
									btnClasses="p-0"
									on:click={() => values.columns.splice(i, 1)}
								/>
							</Cell>
						</tr>
					{/each}
				</tbody>
			</DataTable>
		</div>
	</div>
	<Button disabled={!!errors} on:click={() => onConfirm(values)}>Create table</Button>
</div>
