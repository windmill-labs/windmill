<script lang="ts">
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { getPrimaryKeys, type ColumnDef, type DbType } from './utils'
	import { sendUserToast } from '$lib/toast'
	import { getDeleteInput } from './queries/delete'

	interface Props {
		id: string
	}

	let { id }: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_delete`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state()
	let loading = $state(false)

	let input: AppInput | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	export async function triggerDelete(
		resource: string,
		table: string,
		allColumns: ColumnDef[],
		data: Record<string, any>,
		dbType: DbType
	) {
		let primaryColumns = getPrimaryKeys(allColumns)
		let columns = allColumns?.filter((x) => primaryColumns.includes(x.field))

		input = getDeleteInput(resource, table, columns, dbType)

		await tick()

		if (runnableComponent) {
			let ndata = {}
			columns.forEach((x) => {
				ndata[x.field] = data[x.field]
			})
			await runnableComponent?.runComponent(
				undefined,
				undefined,
				undefined,
				{ ...ndata },
				{
					onDone: (_x) => {
						sendUserToast('Row deleted', false)
						dispatch('deleted')
					},
					onCancel: () => {
						sendUserToast('Error deleting row', true)
					},
					onError: () => {
						sendUserToast('Error updating row', true)
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
	id={`${id}_delete`}
	{outputs}
/>
