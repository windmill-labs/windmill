<script lang="ts">
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef, type DbType } from './utils'
	import { sendUserToast } from '$lib/toast'
	import { getInsertInput } from './queries/insert'

	interface Props {
		id: string
	}

	let { id }: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_insert`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state()
	let loading = $state(false)
	let input: AppInput | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	export async function insertRow(
		resource: string,
		workspace: string | undefined,
		table: string | undefined,
		columns: ColumnDef[],
		values: Record<string, any>,
		resourceType: string
	): Promise<boolean> {
		if (!resource || !table || !workspace) {
			return false
		}

		input = getInsertInput(table, columns, resource, resourceType as DbType)

		await tick()

		if (runnableComponent) {
			await runnableComponent?.runComponent(undefined, undefined, undefined, values, {
				onDone: (_x) => {
					dispatch('insert')
					sendUserToast('Row inserted', false)
				},
				onCancel: () => {
					sendUserToast('Error inserting row', true)
				},
				onError: () => {
					sendUserToast('Error inserting row', true)
				}
			})
		}

		return false
	}
</script>

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_insert`}
	{outputs}
/>
