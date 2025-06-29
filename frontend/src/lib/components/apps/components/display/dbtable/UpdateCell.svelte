<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { getPrimaryKeys, type ColumnDef, type DbType } from './utils'
	import { sendUserToast } from '$lib/toast'
	import { getUpdateInput } from './queries/update'

	interface Props {
		id: string
	}

	let { id }: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_update`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state(undefined)
	let loading = $state(false)
	let input: AppInput | undefined = $state(undefined)

	export async function triggerUpdate(
		resource: string,
		table: string,
		column: ColumnDef,
		allColumns: ColumnDef[],
		valueToUpdate: string,
		data: Record<string, any>,
		oldValue: string | undefined = undefined,
		dbType: DbType
	) {
		// const datatype = tableMetaData?.find((column) => column.isprimarykey)?.datatype

		let primaryColumns = getPrimaryKeys(allColumns)
		let columns = allColumns?.filter((x) => primaryColumns.includes(x.field))

		input = getUpdateInput(resource, table, column, columns, dbType)

		await tick()

		if (runnableComponent) {
			let ndata = {}
			columns.forEach((x) => {
				ndata[x.field] = data[x.field]
			})
			ndata[column.field] = oldValue
			await runnableComponent?.runComponent(
				undefined,
				undefined,
				undefined,
				{ value_to_update: valueToUpdate, ...ndata },
				{
					done: (x) => {
						sendUserToast('Value updated', false)
					},
					cancel: () => {
						sendUserToast('Error updating value', true)
					},
					error: () => {
						sendUserToast('Error updating value', true)
					}
				}
			)
		}
	}
</script>

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_update`}
	{outputs}
/>
