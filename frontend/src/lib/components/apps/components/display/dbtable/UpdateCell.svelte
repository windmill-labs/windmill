<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { createUpdatePostgresInput, type ColumnMetadata, getPrimaryKeys } from './utils'
	import { sendUserToast } from '$lib/toast'

	export let id: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_update`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let loading = false

	let input: AppInput | undefined = undefined

	export async function triggerUpdate(
		resource: string,
		table: string,
		column: ColumnMetadata,
		allColumns: ColumnMetadata[],
		valueToUpdate: string,
		data: Record<string, any>,
		oldValue: string | undefined = undefined
	) {
		// const datatype = tableMetaData?.find((column) => column.isprimarykey)?.datatype

		let primaryColumns = getPrimaryKeys(allColumns)
		let columns = allColumns?.filter((x) => primaryColumns.includes(x.field))

		input = createUpdatePostgresInput(resource, table, column, columns)

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
				{ valueToUpdate, ...ndata },
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
